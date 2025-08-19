use crate::*;
use std::{collections::HashMap, sync::Arc, vec};

use cgmath::Quaternion;
use cgmath::*;
use sdl3::{Sdl, VideoSubsystem};
use slotmap::{new_key_type, SlotMap};
use std::sync::RwLock;
use wgpu::{*};
use anyhow;

pub struct RenderContext { // shared instance across windows
    pub sdl_context: Arc<Sdl>,
    pub video_subsystem: Arc<VideoSubsystem>,
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
} impl RenderContext {
    pub async fn new(sdl_context: Arc<Sdl>) -> anyhow::Result<Self> {
        let video_subsystem = Arc::new(sdl_context.video()?);

        let instance = Arc::new(
            Instance::new(&InstanceDescriptor {
                backends: Backends::PRIMARY,
                ..Default::default()
            })
        );

        let adapter = Arc::new(
            instance.request_adapter(
                &RequestAdapterOptions {
                    power_preference: PowerPreference::LowPower,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                },
            ).await.ok_or(anyhow::anyhow!("Failed to get adapter"))?
        ); 

        let (d, q) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;

        let device = Arc::new(d);
        let queue = Arc::new(q);

        Ok(Self { 
            sdl_context, 
            video_subsystem, 
            instance, 
            adapter, 
            device,
            queue, 
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)] pub enum BlendMode { None, Premultiplied, AlphaBlend, Additive, Multiply, Subtract }
#[derive(Clone, Copy, PartialEq, Eq)] pub enum FaceCullMode { None, Back, Front }
#[derive(Copy, Clone, PartialEq, Eq)] pub enum AntiAliasing { None, MSAA2x, MSAA4x, MSAA8x, FXAA, SMAA }
#[derive(Copy, Clone, PartialEq, Eq)] pub enum FilterMode { Nearest, Linear }
#[derive(Copy, Clone, PartialEq, Eq)] pub enum WrapMode { Repeat, MirroredRepeat, Clamp }

#[derive(Clone)]
pub struct Material {
    pub textures: Vec<TextureKey>,
    pub shader: ShaderKey,
    
    pub blend_mode: BlendMode,
    pub cull_mode: FaceCullMode,
    pub filter_mode: FilterMode,
    pub wrap_mode: (WrapMode, WrapMode), // h, v
}

#[derive(Clone, Copy)]
pub enum Mapping {
    Sprite {
        uv_rect: Rectangle<f32>,
    },
    Mesh {
        vertex_buffer: VertexBufferKey,
        index_buffer: IndexBufferKey,
        vertex_count: u32,
        index_count: u32,
    },
}

// [Rx Ry Rz Tx]
// [Rx Ry Rz Ty]
// [Rx Ry Rz Tz]
// [ 0  0  0  1]
#[derive(Clone, Copy)]
pub enum Transform {
    Sprite {
        position: Vector2<f32>,
        rotation: f32,
        scale: Vector2<f32>,
        z_order: f32,
    },
    Mesh {
        position: Vector3<f32>,
        rotation: Quaternion<f32>,
        scale: Vector3<f32>,
    },
} impl Transform {
    pub fn to_matrix(&self) -> Matrix4<f32> {
        match self {
            Transform::Sprite { position, rotation, scale, z_order } => {
                Matrix4::from_translation(Vector3::new(position.x, position.y, *z_order))
                * Matrix4::from_angle_z(Rad(*rotation))
                * Matrix4::from_nonuniform_scale(scale.x, scale.y, 1.0)                
            }
            Transform::Mesh { position, rotation, scale } => {
                Matrix4::from_translation(*position)
                * Matrix4::from(*rotation)
                * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)                
            }
        }
    }

    pub fn depth(&self) -> f32 {
        match self {
            Transform::Sprite { z_order, .. } => *z_order,
            Transform::Mesh { position, .. } => position.z,
        }
    }
}

#[derive(Clone)]
pub struct MaterialUniforms {
    pub tint: Vector4<f32>,
    pub custom_params: Vec<f32>, // Shader-specific parameters
}

#[derive(Debug, Clone)]
pub enum RenderTargetKey {
    Screen, 
    Texture(TextureKey),
}

#[derive(Debug, Clone)]
pub enum ScheduleStep {
    Pass {
        render_pass: RenderPassName,
        target: RenderTargetName,
    },
    Process {
        subject: RenderTargetName,
        shader: ShaderKey,
        target: RenderTargetName,
    }
}

#[derive(Debug)]
pub struct RenderSchedule {
    pub steps: Vec<ScheduleStep>,
    pub pass_names: Vec<RenderPassName>,
    pub render_targets: HashMap<RenderTargetName, RenderTargetKey>,
} impl RenderSchedule {
    fn new(mut render_targets: HashMap<RenderTargetName, RenderTargetKey>) -> Self {
        render_targets.insert(
            "screen".to_string(), 
            RenderTargetKey::Screen
        );
        Self {
            steps: vec![],
            pass_names: vec![],
            render_targets
        }
    }

    pub fn builder() -> Self {
        Self::new(HashMap::new())
    }

    pub fn with_render_target(mut self, name: impl Into<String>, target: RenderTargetKey) -> Self {
        self.render_targets.insert(name.into(), target);
        self
    }
    
    pub fn add_pass(self, pass_name: impl Into<String>, target: impl Into<String>) -> Self {
        self.add_step(ScheduleStep::Pass {
            render_pass: pass_name.into(),
            target: target.into(),
        })
    }

    pub fn add_process(self, subject: impl Into<String>, shader: ShaderKey, target: impl Into<String>) -> Self {
        self.add_step(ScheduleStep::Process {
            subject: subject.into(),
            shader,
            target: target.into(),
        })
    }

    fn add_step(mut self, step: ScheduleStep) -> Self {
        match &step { // Use reference to avoid clone
            ScheduleStep::Pass { render_pass, target } => {
                if !self.pass_names.contains(render_pass) { 
                    self.pass_names.push(render_pass.clone()) 
                };
                if !self.render_targets.contains_key(target) { 
                    panic!("Render target '{}' not found", target) 
                } 
            }
            ScheduleStep::Process { subject, target, .. } => {
                if !self.render_targets.contains_key(subject) { 
                    panic!("Subject render target '{}' not found", subject) 
                }
                if !self.render_targets.contains_key(target) { 
                    panic!("Target render target '{}' not found", target) 
                } 
            }
        }

        self.steps.push(step);
        self
    }
}

#[derive(Clone)]
pub struct RenderQueue {
    material: MaterialKey,
    mapping: Mapping,
    transform: Transform,
    uniforms: MaterialUniforms,
    allow_transparency: bool,
    queue_depth: f32,
}

pub struct Renderer { // one per window
    pub render_context: Arc<RenderContext>,
    pub assets: Arc<AssetManager>,

    pub schedule: RenderSchedule,
    pub queues: HashMap<RenderPassName, (Vec<RenderQueue>, Vec<RenderQueue>)>, // 0: opaque (batched), 1: allow transparency
    
    depth_counter: f32,
} impl Renderer {
    pub fn new(render_context: Arc<RenderContext>, assets: Arc<AssetManager>) -> Self {
        let mut renderer = Self {
            render_context,
            assets,
            
            schedule: RenderSchedule::new(HashMap::new()),
            queues: HashMap::new(),
            depth_counter: 0.0,
        };

        renderer
    }

    pub fn create_dynamic_render_target(&mut self, size: (u32, u32), name: &str) -> RenderTargetKey {
        if let Some(existing) = self.assets.dynamic_render_targets.read().unwrap().get(name) {
            return existing.clone();
        }

        let texture = SQTexture::new(self.render_context.device.clone(), size);
        let texture_key = self.assets.textures.write().unwrap().insert(texture);
        let render_target_key = RenderTargetKey::Texture(texture_key);
        
        self.assets.dynamic_render_targets.write().unwrap().insert(
            name.to_owned(), 
            render_target_key.clone(),
        );
        
        render_target_key
    }

    pub fn queue(
        &mut self,
        material: MaterialKey,
        mapping: Mapping,
        transform: Transform,
        uniforms: MaterialUniforms,
        pass_name: RenderPassName,
        allow_transparency: bool,
    ) {
        let queue = RenderQueue {
            material, 
            mapping, 
            transform,
            uniforms,
            allow_transparency,
            queue_depth: self.depth_counter, 
        };
        self.depth_counter += 1.0;

        let entry = self.queues.entry(pass_name.clone()).or_insert_with(|| (Vec::new(), Vec::new()));
        if allow_transparency {
            entry.1.push(queue);
        } else {
            entry.0.push(queue);
        }
    }

    pub fn execute(&mut self) {
        for step in &self.schedule.steps.clone() {
            match step {
                ScheduleStep::Pass { render_pass, target } => {
                    if let Some(queues) = self.queues.remove(render_pass) {
                        self.render_batched(target, queues.0);
                        self.render_transparent(target, queues.1);
                    }
                }
                ScheduleStep::Process { subject, shader, target } => {
                    let subject_texture = &self.schedule.render_targets[subject];
                    let shader_pipeline = &self.assets.shaders.read().unwrap()[*shader];
                    self.execute_post_process();
                }
            }
        }

        self.queues.clear();
        self.depth_counter = 0.0;
    }
}
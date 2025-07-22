#![allow(unused, dead_code)]
mod create_surface;
mod structs; use structs::*;
mod texture; use texture::*;
mod input; use input::*;
mod screen; use screen::*;
mod camera; use camera::*;

extern crate sdl3; use sdl3::{*, event::*, video::*};
extern crate wgpu; use wgpu::*;
use cgmath::prelude::*;

use slotmap::SlotMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use util::DeviceExt;

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5);

pub struct App {
    running: bool,
    event_pump: EventPump,
    render_context: RenderContext,
    windows: HashMap<u32, RenderWindow>,
    
    screens: ScreenManager,
    // assets: AssetManager,
    // audios: AudioManager,
} impl App {
    pub async fn new(sdl_context: Sdl) -> Self {
        let sdl_context = sdl3::init().unwrap();
        let event_pump = sdl_context.event_pump().unwrap();
        let render_context = RenderContext::new(sdl_context).await;

        Self {
            running: true,
            event_pump,
            render_context,
            windows: todo!(),
            screens: ScreenManager::new(),
        }
    }

    pub async fn run(&mut self) {
        'running: loop {
            self.update();
            if (!self.running) {
                break 'running
            }
        }
    }

    fn get_window_id(event: &Event) -> Option<u32> {
        match event {
            Event::Window { window_id, .. }
            | Event::KeyDown { window_id, .. }
            | Event::KeyDown { window_id, .. }
            | Event::MouseMotion { window_id, .. }
            | Event::MouseButtonDown { window_id, .. }
            | Event::MouseButtonUp { window_id, .. }
            | Event::MouseWheel { window_id, .. }
            | Event::TextInput { window_id, ..}
            | Event::TextEditing { window_id, ..}
            | Event::DropBegin { window_id, .. }
            | Event::DropText { window_id, .. }
            | Event::DropFile { window_id, ..}
            | Event::DropComplete { window_id, .. }  => Some(*window_id),
            _ => None,
        }
    }

    fn get_window(window_id: u32) {

    }

    pub fn update(&mut self) {
        self.handle_events();

        // todo: game logic

        self.render();
    }

    fn handle_events(&mut self) {
        let events: Vec<_> = self.event_pump.poll_iter().collect();
        for event in events { // window events
            if let Some(window_id) = App::get_window_id(&event) {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.handle_event(&event.clone(), &self.render_context);
                }
            } else { // global 
                match event {
                    Event::Quit {..} => { self.running = false; },
                    _ => {}
                }
            }
        }        
    }

    fn render(&mut self) {
        for (_, window) in self.windows.iter_mut() {

            match window.render(&self.render_context) {
                Ok(_) => {}
                Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                    let size = window.window.size();
                    window.resize(&self.render_context.device, size.0, size.1);
                }
                Err(e) => {
                    println!("Unable to render {}", e);
                }
            }
        }
    }
} impl Default for App {
    fn default() -> Self {
        Self::new()
    }
} 

pub struct RenderContext {
    pub sdl_context: Sdl,
    pub instance: Arc<Instance>,
    pub device: Device,
    pub video_subsystem: VideoSubsystem,
    pub queue: Queue,
    pub pipelines: SlotMap<RenderPipelineKey, RenderPipeline>,
    pub materials: SlotMap<MaterialKey, Material>,
    // pub meshes: MeshManager,
} impl RenderContext {
    pub async fn new(sdl_context: Sdl) -> anyhow::Result<Self> {
        // instance is the first thing we want to create with wgpu
        // it creates Adapters and Surfaces
        let instance = Arc::new(Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        }));
        
        // adapter is a handle for our actual graphics card
        // used to create Device and Queue
        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: todo!(),
            },
        ).await.unwrap(); 

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::default(),
                memory_hints: Default::default(),
            }, None,
        ).await?;

        let video_subsystem = sdl_context.video().unwrap();

        Ok(sdl_context, instance, device, video_subsystem, queue)
    }
}

pub struct RenderWindow {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    
    pub inputs: InputManager,
    pub camera: Camera,
    pub dirty: bool,
} impl RenderWindow {
    pub fn new(render_context: &RenderContext) -> anyhow::Result<Self> {
        let window = render_context.video_subsystem.window("sq", 800, 600)
            .position_centered()
            .build()
            .unwrap();
        let size = window.size();

        let surface =  create_surface::create_surface(instance.clone(), window.clone()).unwrap();
        
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // SurfaceConfiguration defines how the surface creates its underlying SurfaceTextures
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0,
            height: size.1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);


        Ok(Self { window, surface, config, inputs, camera, dirty: false })
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(device, &self.config);
            self.dirty = true;
        }
    }

    pub fn handle_event(&mut self, event: &Event, render_context: &RenderContext) {
        match event {
            Event::KeyDown { keycode: Some(code), .. } => {
            }
            Event::KeyUp { keycode: Some(code), .. } => {
            }
            Event::Window { win_event: WindowEvent::Resized(width, height), .. } => {
                self.resize(&app.render_context.device, *width as u32, *height as u32);
            }
            _ => {}
        }
    }

    fn render(&self, render_context: &RenderContext) -> Result<(), SurfaceError> {
        // if !self.is_surface_configured {
        //     return Ok(());
        // }
        // wait for the surface to provide a new SurfaceTexture to render to
        let surface_texture = self.surface.get_current_texture()?;
        let texture_view = surface_texture.texture.create_view(&TextureViewDescriptor::default());

        // CommandEncoder builds a command buffer to send to the GPU
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            // RenderPassDescriptor only has three fields: label, color_attachments and depth_stencil_attatchment
            // color_attachments describe where to draw color to, we use texture_view so that we draw to the screen
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None, 
                    ops: Operations {
                        load: LoadOp::Clear(Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0}),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        } // drop render_poss to release &mut encoder so that we can finish it

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
 }

slotmap::new_key_type! { pub struct RenderPipelineKey; }

slotmap::new_key_type! { pub struct MaterialKey; }
pub struct Material {
    bind_group: BindGroup,
}

pub struct AppState {
    context: Arc<Sdl>,
    
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    diffuse_bind_group: BindGroup,
    diffuse_texture: texture::Texture,
    instances: Vec<ModelInstance>,
    instance_buffer: Buffer,

    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,

    render_pipeline: RenderPipeline,
    is_surface_configured: bool,
} impl AppState {
    pub async fn new(window: Arc<Window>, context: Arc<Sdl>) -> anyhow::Result<Self> {

        let diffuse_bytes = include_bytes!("../happy-tree.png");
        let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "../happy-tree.png").unwrap();

        let texture_bind_group_layout = 
            device.create_bind_group_layout((&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float  {  filterable: true  },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }));

        let diffuse_bind_group = device.create_bind_group(
            &BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&diffuse_texture.view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&diffuse_texture.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );
        

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
        
        let vertex_buffer = device.create_buffer_init(
            &util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &util::BufferInitDescriptor {
                label: Some("Index Buffer"), 
                contents: bytemuck::cast_slice(INDICES),
                usage: BufferUsages::INDEX,
            }
        );

        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let position = cgmath::Vector3 { x: x as f32, y: 0.0, z: z as f32 } - INSTANCE_DISPLACEMENT;

                let rotation = if position.is_zero() {
                    // this is needed so an object at (0, 0, 0) won't get scaled to zero
                    // as Quaternions can affect scale if they're not created correctly
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                };

                ModelInstance {
                    position, rotation,
                }
            })
        }).collect::<Vec<_>>();

        let instance_data = instances.iter().map(ModelInstance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
        &util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: BufferUsages::VERTEX,
            }
        );
        
        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let camera_controller = CameraController::new(0.01);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(
            &util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }
        );
        
        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
                ],
                label: Some("camera_bind_group_layout"),
            });
            
        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });       

        let render_pipeline_layout = device.create_pipeline_layout(
            &PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            }
        );

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc(),
                    ModelInstanceRaw::desc(),
                ],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            window, 
            context, 
            instance, 
            surface, 
            device, 
            queue, 
            config, 
            vertex_buffer, 
            index_buffer, 
            diffuse_bind_group, 
            diffuse_texture, 
            instances,
            instance_buffer,
            camera, 
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            render_pipeline, 
            is_surface_configured: true, 
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    fn input(&mut self, event: &Event) {
        self.camera_controller.process_events(event);
    }

    

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    
}
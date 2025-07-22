#![allow(unused, dead_code)]
mod create_surface;
mod structs; use rodio::cpal::FromSample;
use structs::*;
mod texture; use texture::*;
mod input; use input::*;
mod screen; use screen::*;
mod camera; use camera::*;

extern crate sdl3; use sdl3::{*, event::*, video::*, pen::*};
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
    pub async fn new() -> anyhow::Result<Self> {
        let sdl_context = sdl3::init().unwrap();
        let event_pump = sdl_context.event_pump().unwrap();
        let render_context = RenderContext::new(sdl_context).await?;

        Ok(Self {
            running: true,
            event_pump,
            render_context,
            windows: HashMap::new(),
            screens: ScreenManager::new(),
        })
    }

    pub async fn run(&mut self) {
        'running: loop { // todo: framerates
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
            | Event::KeyUp { window_id, .. }
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
            Event::PenMotion { window, .. }
            | Event::PenUp { window, .. }
            | Event::PenDown { window, .. }
            | Event::PenButtonDown { window, .. }
            | Event::PenButtonUp { window, .. }
            | Event::PenProximityIn { window, .. }
            | Event::PenProximityOut { window, .. } => Some(*window),
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

        for (_, window) in self.windows.iter_mut() {
            window.inputs.update(&self.screens);
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
}

pub struct RenderContext {
    pub sdl_context: Sdl,
    pub instance: Arc<Instance>,
    pub adapter: Adapter,
    pub device: Device,
    pub video_subsystem: VideoSubsystem,
    pub queue: Queue,
    pub pipelines: SlotMap<RenderPipelineKey, RenderPipeline>,
    pub materials: SlotMap<MaterialKey, Material>,
    // pub meshes: MeshManager,
} impl RenderContext {
    pub async fn new(sdl_context: Sdl) -> anyhow::Result<Self> {
        let instance = Arc::new(Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        }));
        
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

        Ok(Self { sdl_context, instance, adapter, device, video_subsystem, queue, pipelines: SlotMap::with_key(), materials: SlotMap::with_key() })
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
        let window = Arc::new(render_context.video_subsystem.window("sq", 800, 600)
            .position_centered()
            .build()
            .unwrap());
        let size = window.size();

        let surface =  create_surface::create_surface(render_context.instance.clone(), window.clone()).unwrap();
        
        let surface_caps = surface.get_capabilities(&render_context.adapter);
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

        surface.configure(&render_context.device, &config);
        
        let inputs = InputManager::new();
        let camera = todo!();

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
            Event::KeyDown {..} | Event::KeyUp {..} | Event::MouseButtonDown {..} | Event::MouseButtonUp {..} | Event::MouseMotion {..} | Event::MouseWheel {..} => {
                self.inputs.handle_event(event);
            }
            Event::Window { win_event: WindowEvent::Resized(width, height), .. } => {
                self.resize(&render_context.device, *width as u32, *height as u32);
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
        let mut encoder = render_context.device.create_command_encoder(&CommandEncoderDescriptor {
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

            // render_pass.set_pipeline(&self.render_pipeline);
            // render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            // render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            // render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            // render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        } // drop render_poss to release &mut encoder so that we can finish it

        render_context.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
 }

slotmap::new_key_type! { pub struct RenderPipelineKey; }

slotmap::new_key_type! { pub struct MaterialKey; }
pub struct Material {
    bind_group: BindGroup,
}

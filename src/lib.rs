#![allow(unused, dead_code)]
mod create_surface;
mod structs; use structs::*;

extern crate sdl3;
extern crate wgpu;

use std::sync::Arc;
use std::time::Duration;
use sdl3::{
    event::*, keyboard::Keycode, video::Window, EventPump, Sdl
};
use wgpu::util::DeviceExt;

pub struct App {
    state: Option<AppState>,
} impl App {
    pub fn new() -> Self {
        Self {
            state: None,
        }
    }
} impl Default for App {
    fn default() -> Self {
        Self::new()
    }
} 

pub struct AppState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    context: Arc<Sdl>,
    event_pump: EventPump,
    
    instance: Arc<wgpu::Instance>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    is_surface_configured: bool,
    running: bool,
} impl AppState {
    pub async fn new(window: Arc<Window>, context: Arc<Sdl>) -> anyhow::Result<Self> {
        let size = window.size();
        let event_pump = context.event_pump().unwrap();

        // instance is the first thing we want to create with wgpu
        // it creates Adapters and Surfaces
        let instance = Arc::new(wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        }));
        
        let surface =  create_surface::create_surface(instance.clone(), window.clone()).unwrap();

        // adapter is a handle for our actual graphics card
        // used to create Device and Queue
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false
            },
        ).await.unwrap(); 

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            }, None,
        ).await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // SurfaceConfiguration defines how the surface creates its underlying SurfaceTextures
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Fifo, 
            alpha_mode: surface_caps.alpha_modes[0], // transparent windows todo here
            view_formats: vec![], // list of TextureFormats that can be used when creating TextureViews
            desired_maximum_frame_latency: 2,
        }; 

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {window, context, event_pump, instance, surface, device, queue, config, vertex_buffer, render_pipeline, is_surface_configured: true, running: true})
    }

    pub fn window(&self) -> &Window {
        &self.window
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
        todo!()
    }

    fn window_event(&mut self) {
        let events: Vec<_> = self.event_pump.poll_iter().collect();
        for event in events {
            match event {
                Event::KeyDown { keycode: Some(code), .. } => {
                }
                Event::KeyUp { keycode: Some(code), .. } => {
                }
                Event::Window { win_event: WindowEvent::Resized(width, height), .. } => {
                    self.resize(width as u32, height as u32);
                }
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), ..} => { self.running = false; },
                _ => {}
            }
        }

        match self.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let size = self.window.size();
                self.resize(size.0, size.1);
            }
            Err(e) => {
                println!("Unable to render {}", e);
            }
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // if !self.is_surface_configured {
        //     return Ok(());
        // }
        // wait for the surface to provide a new SurfaceTexture to render to
        let surface_texture = self.surface.get_current_texture()?;
        let texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // CommandEncoder builds a command buffer to send to the GPU
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            // RenderPassDescriptor only has three fields: label, color_attachments and depth_stencil_attatchment
            // color_attachments describe where to draw color to, we use texture_view so that we draw to the screen
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None, 
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0}),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..VERTICES.len() as u32, 0..1);
        } // drop render_poss to release &mut encoder so that we can finish it

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}

pub async fn run() {
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("sqgui", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut app_state = AppState::new(Arc::new(window), Arc::new(sdl_context.clone())).await.unwrap();
    let mut surface_configured = false;

    'running: loop {
        app_state.window_event();
        if (!app_state.running) {
            break 'running
        }
    }
}
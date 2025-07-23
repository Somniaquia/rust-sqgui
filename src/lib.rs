#![allow(unused, dead_code)]
mod create_surface;
mod structs; use structs::*;
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
        let sdl_context = sdl3::init()?;
        let event_pump = sdl_context.event_pump()?;
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

    pub async fn create_window(&mut self, title: &str, width: u32, height: u32) -> anyhow::Result<u32> {
        let window = RenderWindow::new(&self.render_context, title, width, height)?;

        let window_id = window.window.id();
        self.windows.insert(window_id, window);
        Ok(window_id)
    }

    pub fn close_window(&mut self, window_id: u32) -> anyhow::Result<()> {
        if let Some(window) = self.windows.remove(&window_id) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to close window"))
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

slotmap::new_key_type! { pub struct RenderPipelineKey; }

slotmap::new_key_type! { pub struct MaterialKey; }
pub struct Material {
    bind_group: BindGroup,
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
} impl RenderContext {
    pub async fn new(sdl_context: Sdl) -> anyhow::Result<Self> {
        let video_subsystem = sdl_context.video()?;

        let temp_window = Arc::new(video_subsystem.window("temp", 1, 1)
            .hidden()
            .build()?);

        let instance = Arc::new(Instance::new(&InstanceDescriptor {
            backends: Backends::SECONDARY,
            ..Default::default()
        }));

        let temp_surface = unsafe {
            create_surface::create_surface(instance.clone(), temp_window.clone())?
        };
        
        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: Some(&temp_surface), // surfaces, binded to windows are created AFTER context, what do i do here
            },
        ).await.ok_or(anyhow::anyhow!("Failed to get adapter"))?; 

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor::default(), 
            None,
        ).await?;

        drop(temp_surface);
        drop(temp_window);

        Ok(Self { 
            sdl_context, 
            instance, 
            adapter, 
            device,
            video_subsystem, 
            queue, 
            pipelines: SlotMap::with_key(), 
            materials: SlotMap::with_key()
        })
    }
}

pub struct RenderWindow {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    
    pub inputs: InputManager,
    pub dirty: bool,
} impl RenderWindow {
    pub fn new(render_context: &RenderContext, title: &str, width: u32, height: u32) -> anyhow::Result<Self> {
        let window = Arc::new(render_context.video_subsystem.window("sq", 800, 600)
            .position_centered()
            .build()?
        );

        let surface = create_surface::create_surface(render_context.instance.clone(), window.clone())?;
        
        let caps = surface.get_capabilities(&render_context.adapter);
        let format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        // SurfaceConfiguration defines how the surface creates its underlying SurfaceTextures
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&render_context.device, &config);

        Ok(Self { 
            window, 
            surface, 
            config, 
            inputs: InputManager::new(), 
            dirty: false 
        })
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
        // todo: definable pipeline
        Ok(())
    }
 }

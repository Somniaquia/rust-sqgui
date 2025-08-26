use anyhow::*;
use cgmath::Vector2;
use sdl3::{event::*, video::Window, *};
use sq::*;
use wgpu::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app = SQ::new().await?;

    app.create_window("sq", 800, 600).await?;
    app.run().await;
    Ok(())
}

pub struct SQ {
    pub sdl_context: Arc<Sdl>,
    pub event_pump: EventPump,
    pub render_context: Arc<RenderContext>,
    pub assets: Arc<AssetManager>,
    pub windows: HashMap<u32, SQWindow>,
    pub running: bool,
}
impl SQ {
    pub async fn new() -> Result<Self> {
        let sdl_context = Arc::new(sdl3::init()?);
        let event_pump = sdl_context.event_pump()?;
        let render_context = Arc::new(RenderContext::new(sdl_context.clone()).await?);
        let assets = Arc::new(AssetManager::new());

        Ok(Self {
            sdl_context,
            event_pump,
            render_context,
            assets,
            windows: HashMap::new(),
            running: true,
        })
    }

    pub async fn run(&mut self) {
        'running: loop {
            if (!self.running) {
                break 'running;
            }
            let frame_start = std::time::Instant::now();

            self.update();
            self.render();

            let frame_time = frame_start.elapsed();
            let target_frame_time = Duration::from_secs_f32(1.0 / 60.0);
            if frame_time < target_frame_time {
                std::thread::sleep(target_frame_time - frame_time);
            }
        }
    }

    pub async fn create_window(&mut self, title: &str, width: u32, height: u32) -> Result<u32> {
        let window = SQWindow::new(
            self.render_context.clone(),
            self.assets.clone(),
            title,
            width,
            height,
        )?;

        let window_id = window.window.id();
        self.windows.insert(window_id, window);
        Ok(window_id)
    }

    pub fn close_window(&mut self, window_id: u32) -> Result<()> {
        if let Some(window) = self.windows.remove(&window_id) {
            Ok(())
        } else {
            Err(anyhow!("Failed to close window"))
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
            | Event::TextInput { window_id, .. }
            | Event::TextEditing { window_id, .. }
            | Event::DropBegin { window_id, .. }
            | Event::DropText { window_id, .. }
            | Event::DropFile { window_id, .. }
            | Event::DropComplete { window_id, .. } => Some(*window_id),
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

    fn get_window(window_id: u32) {}

    pub fn update(&mut self) {
        self.handle_events();

        // todo: game logic
    }

    fn handle_events(&mut self) {
        let events: Vec<_> = self.event_pump.poll_iter().collect();
        for event in events {
            // window events
            if let Some(window_id) = SQ::get_window_id(&event) {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.handle_event(&event.clone(), &self.render_context);
                }
            } else {
                // global
                match event {
                    Event::Quit { .. } => {
                        self.running = false;
                    }
                    _ => {}
                }
            }
        }

        for (_, window) in self.windows.iter_mut() {
            // window.inputs.update(&self.screens);
        }
    }

    fn render(&mut self) -> Result<()> {
        for (_, window) in self.windows.iter_mut() {
            match window.render(&self.render_context) {
                Ok(_) => {}
                Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                    let size = window.window.size();
                    window.resize(&self.render_context.device, size.0, size.1);
                    window.render(&self.render_context)?;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }
}

pub struct SQWindow {
    pub window: Arc<Window>,
    pub renderer: Renderer,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,

    pub inputs: InputManager,

    pub size: Vector2<f32>,
    pub focused: bool,
    pub minimized: bool,
}
impl SQWindow {
    pub fn new(
        render_context: Arc<RenderContext>,
        assets: Arc<AssetManager>,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let window = Arc::new(
            render_context
                .video_subsystem
                .window(title, width, height)
                .high_pixel_density()
                .position_centered()
                .borderless()
                .resizable()
                .vulkan()
                .build()?,
        );

        let renderer = Renderer::new(render_context.clone(), assets);

        let surface =
            create_surface::create_surface(render_context.instance.clone(), window.clone())?;

        let caps = surface.get_capabilities(&render_context.adapter);
        let format = caps
            .formats
            .iter()
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
            renderer,
            surface,
            config,
            inputs: InputManager::new(),

            size: Vector2 {
                x: width as f32,
                y: height as f32,
            },
            focused: true,
            minimized: false,
        })
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.size = Vector2 {
                x: width as f32,
                y: height as f32,
            };
            self.surface.configure(device, &self.config);
        }
    }

    pub fn handle_event(&mut self, event: &Event, render_context: &RenderContext) {
        match event {
            Event::Window {
                win_event: WindowEvent::Resized(width, height),
                ..
            } => {
                self.resize(&render_context.device, *width as u32, *height as u32);
            }
            _ => self.inputs.handle_event(event),
        }
    }

    fn render(&self, render_context: &RenderContext) -> Result<(), SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let texture_view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        // CommandEncoder builds a command buffer to send to the GPU
        let mut encoder = render_context
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
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
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
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

        render_context
            .queue
            .submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}

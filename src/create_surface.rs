use std::sync::Arc;

use sdl3::video::Window;
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};

// contains the unsafe impl as much as possible by putting it in this module
struct SyncWindow<'a>(&'a    Window);

unsafe impl Send for SyncWindow<'_> {}
unsafe impl Sync for SyncWindow<'_> {}

impl HasWindowHandle for SyncWindow<'_> {
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
        self.0.window_handle()
    }
}
impl HasDisplayHandle for SyncWindow<'_> {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        self.0.display_handle()
    }
}

pub fn create_surface(
    instance: Arc<wgpu::Instance>,
    window: Arc<Window>,
) -> anyhow::Result<wgpu::Surface<'static>> {
    // Safety: We're ensuring the window lives as long as the surface
    // by storing it in an Arc
    unsafe {
        let window_ref: &'static Window = std::mem::transmute(&*window);
        instance.create_surface(SyncWindow(window_ref))
            .map_err(|err| anyhow::anyhow!("Failed to create surface: {:?}", err))
    }
}
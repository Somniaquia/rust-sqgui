#![allow(unused, dead_code)]
pub mod create_surface;
pub mod structs;
pub use structs::*;
pub mod texture;
pub use texture::*;
pub mod input;
pub use input::*;
pub mod screen;
pub use screen::*;
pub mod camera;
pub use camera::*;
pub mod render;
pub use render::*;
pub mod assets;
pub use assets::*;

extern crate sdl3;
extern crate wgpu;

pub use anyhow::*;
pub use slotmap::*;
pub use std::collections::HashMap;
pub use std::sync::{Arc, Mutex};
pub use std::time::Duration;

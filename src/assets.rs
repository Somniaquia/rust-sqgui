use crate::*;

use std::{collections::HashMap, sync::RwLock};
use anyhow::Context;
use slotmap::new_key_type;

new_key_type! {
    pub struct TextureKey;
    pub struct ShaderKey;
    pub struct MaterialKey;
    pub struct VertexBufferKey;
    pub struct IndexBufferKey;
}

pub type AssetName = String;
pub type RenderTargetName = String;
pub type RenderPassName = String;

pub struct AssetManager {
    pub texture_assets: RwLock<HashMap<AssetName, TextureKey>>,
    pub shader_assets: RwLock<HashMap<AssetName, ShaderKey>>,
    pub material_assets: RwLock<HashMap<AssetName, MaterialKey>>,
    pub dynamic_render_targets: RwLock<HashMap<RenderTargetName, RenderTargetKey>>,

    pub vertex_buffers: RwLock<SlotMap<VertexBufferKey, VertexBuffer>>,
    pub index_buffers: RwLock<SlotMap<IndexBufferKey, IndexBuffer>>,

    pub textures: RwLock<SlotMap<TextureKey, SQTexture>>,
    pub shaders: RwLock<SlotMap<ShaderKey, RenderPipeline>>,
    pub materials: RwLock<SlotMap<MaterialKey, Material>>,
} impl AssetManager {
    pub fn new() -> Self {
        Self {
            texture_assets: HashMap::new().into(),
            shader_assets: HashMap::new().into(),
            material_assets: HashMap::new().into(),
            dynamic_render_targets: HashMap::new().into(),

            vertex_buffers: SlotMap::with_key().into(),
            index_buffers: SlotMap::with_key().into(),

            textures: SlotMap::with_key().into(),
            shaders: SlotMap::with_key().into(),
            materials: SlotMap::with_key().into(),
        }
    }

    pub fn load_texture(&mut self, render_context: &RenderContext, path: &str) -> anyhow::Result<AssetName> {
        if self.texture_assets.read().unwrap().contains_key(path) {
            return Ok(path.to_string());
        }

        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read texture file: {}", path))?;
        
        let texture = SQTexture::from_bytes(
            &render_context.device, 
            &render_context.queue, 
            &bytes, 
            path
        )?;
        
        let texture_key = self.textures.write().unwrap().insert(texture);
        
        self.texture_assets.write().unwrap().insert(path.to_string(), texture_key);
        Ok(path.to_string())
    }
    
    pub fn get_texture_key(&self, asset_key: &AssetName) -> Option<TextureKey> {
        self.texture_assets.read().unwrap().get(asset_key).copied()
    }

    pub fn create_material(&self, material: Material) -> MaterialKey {
        self.materials.write().unwrap().insert(material)
    }

    pub fn get_material(&self, key: MaterialKey) -> Option<Material> {
        self.materials.read().unwrap().get(key).cloned()
    }
}
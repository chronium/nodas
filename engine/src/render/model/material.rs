use log::info;

use crate::render::{binding, state, texture};

pub struct Material {
    pub(super) name: String,
    pub(super) textures: binding::TextureBinding,
}

impl Material {
    pub fn new(
        state: &state::WgpuState,
        name: &str,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture,
        material_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        info!("Create material {:?}", name);
        Self {
            name: String::from(name),
            textures: binding::TextureBinding::new(
                state,
                Some(name),
                material_layout,
                &[diffuse_texture, normal_texture],
            ),
        }
    }
}

use anyhow::*;

use std::{collections::HashMap, path::Path};

use crate::{
    render::{
        binding, model, state, texture,
        traits::{Binding, DrawModel},
        Layouts,
    },
    transform,
};

use legion::IntoQuery;

#[derive(PartialEq, Eq, Hash)]
pub struct ModelIdent(pub String);
#[derive(PartialEq, Eq, Hash)]
pub struct MaterialIdent(pub String);

pub struct World {
    models: HashMap<ModelIdent, model::Model>,
    materials: HashMap<MaterialIdent, model::Material>,
    world: legion::World,
}

impl World {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            materials: HashMap::new(),
            world: legion::World::new(legion::WorldOptions::default()),
        }
    }

    pub fn load_model<P: AsRef<Path>, M: Into<String>>(
        &mut self,
        state: &state::WgpuState,
        layouts: &Layouts,
        name: M,
        path: P,
    ) -> Result<()> {
        self.models.insert(
            ModelIdent(name.into()),
            model::Model::load(state, &layouts.material, path)?,
        );
        Ok(())
    }

    pub fn load_material_raw(
        &mut self,
        state: &state::WgpuState,
        name: &str,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture,
        material_layout: &wgpu::BindGroupLayout,
    ) {
        self.materials.insert(
            MaterialIdent(name.into()),
            model::Material::new(
                &state,
                name,
                diffuse_texture,
                normal_texture,
                material_layout,
            ),
        );
    }

    pub fn push_entity<T>(&mut self, components: T) -> legion::Entity
    where
        Option<T>: legion::storage::IntoComponentSource,
    {
        self.world.push(components)
    }

    pub fn render<'a>(
        &'a mut self,
        state: &state::WgpuState,
        render_pass: &mut wgpu::RenderPass<'a>,
        uniforms: &'a binding::BufferGroup,
        light: &'a binding::BufferGroup,
    ) {
        let mut models = <(
            &mut transform::Transform,
            &ModelIdent,
            Option<&MaterialIdent>,
        )>::query();

        for (transform, model, material) in models.iter_mut(&mut self.world) {
            render_pass.bind_buffer(1, transform.buffer(state));

            match material {
                Some(material) => {
                    render_pass.draw_model_with_material(
                        &self.models.get(model).expect("Model not found"),
                        &self.materials.get(material).expect("Material not found"),
                        &uniforms,
                        &light,
                    );
                }
                None => {
                    render_pass.draw_model(
                        &self.models.get(model).expect("Model not found"),
                        &uniforms,
                        &light,
                    );
                }
            }
        }
    }
}

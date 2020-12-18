use anyhow::*;

use std::{collections::HashMap, path::Path};

use crate::{
    render::{
        binding, model, state,
        traits::{Binding, DrawModel},
        Layouts,
    },
    transform,
};

use legion::IntoQuery;

#[derive(PartialEq, Eq, Hash)]
pub struct ModelIdent(String);

pub struct World {
    models: HashMap<ModelIdent, model::Model>,
    world: legion::World,
}

impl World {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
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

    pub fn push_entity<M: Into<String>>(
        &mut self,
        model: M,
        transform: transform::Transform,
    ) -> legion::Entity {
        self.world.push((transform, ModelIdent(model.into())))
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        uniforms: &'a binding::BufferGroup,
        light: &'a binding::BufferGroup,
    ) {
        let mut query = <(&transform::Transform, &ModelIdent)>::query();

        for (transform, model) in query.iter(&self.world) {
            render_pass.bind_buffer(1, &transform.buffer);
            render_pass.draw_model(
                &self.models.get(model).expect("Model not found"),
                &uniforms,
                &light,
            );
        }
    }
}

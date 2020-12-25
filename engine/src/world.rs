use anyhow::*;
use model::Model;
use ncollide3d::pipeline::CollisionObjectSlabHandle;

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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModelIdent(pub String);
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MaterialIdent(pub String);
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct CollisionGroup(usize);

pub struct ColliderGroup(Vec<CollisionObjectSlabHandle>);

pub struct World {
    pub models: HashMap<ModelIdent, model::Model>,
    materials: HashMap<MaterialIdent, model::Material>,
    world: legion::World,
    collision_world: ncollide3d::world::CollisionWorld<f32, legion::Entity>,
}

impl World {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            materials: HashMap::new(),
            world: legion::World::new(legion::WorldOptions::default()),
            collision_world: ncollide3d::world::CollisionWorld::new(0.01),
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

    #[allow(unused)]
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

    pub fn push_entity<T>(&mut self, components: T) -> Result<legion::Entity>
    where
        Option<T>: legion::storage::IntoComponentSource,
    {
        let entity = self.world.push(components);
        self.world
            .entry(entity)
            .unwrap()
            .add_component(ColliderGroup(Vec::new()));

        if let Some(mut entry) = self.world.entry(entity) {
            let isometry = entry.get_component::<transform::Transform>()?.isometry();
            let scale = entry.get_component::<transform::Transform>()?.scale();
            let collision_groups = entry
                .get_component::<ncollide3d::pipeline::object::CollisionGroups>()
                .ok()
                .cloned();
            let model_ident = entry.get_component::<ModelIdent>()?;
            let model = self.models.get(model_ident).context("Cannot find model")?;
            let collider_group = entry.get_component_mut::<ColliderGroup>()?;

            for collider in model.mesh_colliders.iter() {
                collider_group.0.push(
                    self.collision_world
                        .add(
                            isometry,
                            ncollide3d::shape::ShapeHandle::new(collider.clone().scaled(&scale)),
                            collision_groups
                                .unwrap_or(ncollide3d::pipeline::object::CollisionGroups::new())
                                .clone(),
                            ncollide3d::pipeline::object::GeometricQueryType::Contacts(0.0, 0.0),
                            entity,
                        )
                        .0,
                );
            }
        }

        Ok(entity)
    }

    pub fn update_entity_world_transform(&mut self, entity: legion::Entity) -> Result<()> {
        if let Some(entry) = self.world.entry(entity) {
            let transform = entry.get_component::<transform::Transform>()?;
            let model_ident = entry.get_component::<ModelIdent>()?;
            let model = self.models.get(model_ident).context("Cannot find model")?;
            let colliders = entry.get_component::<ColliderGroup>()?;

            for (i, handle) in colliders.0.iter().enumerate() {
                self.collision_world
                    .get_mut(*handle)
                    .unwrap()
                    .set_position(transform.isometry());
                self.collision_world.get_mut(*handle).unwrap().set_shape(
                    ncollide3d::shape::ShapeHandle::new(
                        model.mesh_colliders[i].clone().scaled(&transform.scale()),
                    ),
                );
            }
        }

        Ok(())
    }

    pub fn entry(&mut self, entity: legion::Entity) -> Option<legion::world::Entry> {
        self.world.entry(entity)
    }

    pub fn update_collision_world(&mut self) {
        self.collision_world.update();
    }

    pub fn raycast(
        &self,
        ray: &ncollide3d::query::Ray<f32>,
        max_toi: f32,
    ) -> Option<legion::Entity> {
        Some(
            self.collision_world
                .first_interference_with_ray(
                    ray,
                    max_toi,
                    &ncollide3d::pipeline::CollisionGroups::new(),
                )?
                .co
                .data()
                .clone(),
        )
    }

    fn ensure_models_and_materials(&self) -> Result<()> {
        let mut models = <&ModelIdent>::query();
        let mut materials = <&MaterialIdent>::query();

        let models_not_found = models
            .iter(&self.world)
            .map(|model| (model.clone(), self.models.contains_key(model)))
            .filter(|(_, exists)| !exists)
            .collect::<Vec<_>>();
        let materials_not_found = materials
            .iter(&self.world)
            .map(|material| (material.clone(), self.materials.contains_key(material)))
            .filter(|(_, exists)| !exists)
            .collect::<Vec<_>>();

        match (models_not_found.len() > 0, materials_not_found.len() > 0) {
            (true, false) => Err(anyhow!(
                "Could not find models: {:?}",
                models_not_found
                    .iter()
                    .map(|(model, _)| &model.0)
                    .collect::<Vec<_>>(),
            )),
            (false, true) => Err(anyhow!(format!(
                "Could not find materials: {:?}",
                materials_not_found
                    .iter()
                    .map(|(material, _)| &material.0)
                    .collect::<Vec<_>>()
            ))),
            (true, true) => Err(anyhow!(format!(
                "Could not find models: {:?}\nCould not find materials: {:?}",
                models_not_found
                    .iter()
                    .map(|(model, _)| &model.0)
                    .collect::<Vec<_>>(),
                materials_not_found
                    .iter()
                    .map(|(material, _)| &material.0)
                    .collect::<Vec<_>>()
            ))),
            (false, false) => Ok(()),
        }
    }

    pub fn render<'a>(
        &'a mut self,
        state: &state::WgpuState,
        render_pass: &mut wgpu::RenderPass<'a>,
        uniforms: &'a binding::BufferGroup,
        light: &'a binding::BufferGroup,
    ) -> Result<()> {
        if let Err(e) = self.ensure_models_and_materials() {
            return Err(e);
        }

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

        Ok(())
    }
}

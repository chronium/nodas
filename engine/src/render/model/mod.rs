pub mod geometry;
pub mod material;
pub mod vertex;

pub use geometry::{Geometry, Mesh};
pub use material::Material;
pub use vertex::ModelVertex;

use anyhow::*;
use log::info;
use std::{ops::Range, path::Path};

use super::{
    binding, state, texture,
    traits::{Binding, DrawLight, DrawModel},
};

pub struct Model {
    pub geometry: Geometry,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        state: &state::WgpuState,
        material_layout: &wgpu::BindGroupLayout,
        path: P,
    ) -> Result<Self> {
        info!("Load model {:?}", path.as_ref());
        let (obj_models, obj_materials) = tobj::load_obj(path.as_ref(), true)?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        let mut materials = Vec::new();
        for mat in obj_materials {
            let diffuse_path = mat.diffuse_texture;
            let diffuse_texture =
                texture::Texture::load(state, containing_folder.join(diffuse_path), false)?;

            let normal_path = mat.normal_texture;
            let normal_texture =
                texture::Texture::load(state, containing_folder.join(normal_path), true)?;

            materials.push(Material::new(
                state,
                &mat.name,
                diffuse_texture,
                normal_texture,
                material_layout,
            ));
        }

        let geometry = Geometry::new(state, obj_models);

        Ok(Self {
            geometry,
            materials,
        })
    }
}

impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, uniforms, light);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        self.bind_vertex_buffer(0, &mesh.vertex_buffer);
        self.bind_index_buffer(&mesh.index_buffer);
        self.bind_material(0, &material);
        self.bind_group(1, &uniforms);
        self.bind_group(2, &light);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        self.draw_model_instanced(model, 0..1, uniforms, light);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        for mesh in &model.geometry.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), uniforms, light);
        }
    }

    fn draw_model_with_material(
        &mut self,
        model: &'b Model,
        material: &'b Material,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        self.draw_model_instanced_with_material(model, material, 0..1, uniforms, light);
    }

    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b Model,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        for mesh in &model.geometry.meshes {
            self.draw_mesh_instanced(mesh, material, instances.clone(), uniforms, light);
        }
    }

    fn bind_material(&mut self, index: u32, material: &'b Material) {
        self.bind_textures(index, &material.textures);
    }
}

impl<'a, 'b> DrawLight<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, uniforms, light);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        self.bind_vertex_buffer(0, &mesh.vertex_buffer);
        self.bind_index_buffer(&mesh.index_buffer);
        self.bind_group(0, uniforms);
        self.bind_group(1, light);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, uniforms, light);
    }

    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        for mesh in &model.geometry.meshes {
            self.draw_light_mesh_instanced(mesh, instances.clone(), uniforms, light);
        }
    }
}

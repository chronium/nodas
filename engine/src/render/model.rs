use anyhow::*;
use binding::BufferUsage;
use log::info;
use std::{ops::Range, path::Path};

use super::{
    binding::{self, Buffer},
    state, texture,
    traits::{Binding, DrawLight, DrawModel, Vertex},
};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ModelVertex {
    position: cgmath::Vector3<f32>,
    tex_coords: cgmath::Vector2<f32>,
    normal: cgmath::Vector3<f32>,
    tangent: cgmath::Vector3<f32>,
    bitangent: cgmath::Vector3<f32>,
}

unsafe impl bytemuck::Pod for ModelVertex {}
unsafe impl bytemuck::Zeroable for ModelVertex {}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float3,
                },
            ],
        }
    }
}

pub struct Material {
    pub name: String,
    pub textures: binding::TextureBinding,
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

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
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

        let mut meshes = Vec::new();
        for m in obj_models {
            info!("Load mesh {:?}", m.name);
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ]
                    .into(),
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]].into(),
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ]
                    .into(),
                    tangent: [0.0; 3].into(),
                    bitangent: [0.0; 3].into(),
                });
            }

            let indices = &m.mesh.indices;

            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let pos0: cgmath::Vector3<_> = v0.position.into();
                let pos1: cgmath::Vector3<_> = v1.position.into();
                let pos2: cgmath::Vector3<_> = v2.position.into();

                let uv0: cgmath::Vector2<_> = v0.tex_coords.into();
                let uv1: cgmath::Vector2<_> = v1.tex_coords.into();
                let uv2: cgmath::Vector2<_> = v2.tex_coords.into();

                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;

                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;

                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

                vertices[c[0] as usize].tangent = tangent.into();
                vertices[c[1] as usize].tangent = tangent.into();
                vertices[c[2] as usize].tangent = tangent.into();

                vertices[c[0] as usize].bitangent = bitangent.into();
                vertices[c[1] as usize].bitangent = bitangent.into();
                vertices[c[2] as usize].bitangent = bitangent.into();
            }

            let vertex_buffer =
                Buffer::new_init(state, m.name.as_str(), &vertices, BufferUsage::Vertex);
            let index_buffer =
                Buffer::new_init(state, m.name.as_str(), &m.mesh.indices, BufferUsage::Index);

            meshes.push(Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Self { meshes, materials })
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
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), uniforms, light);
        }
    }

    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b Model,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    ) {
        for mesh in &model.meshes {
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
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(mesh, instances.clone(), uniforms, light);
        }
    }
}

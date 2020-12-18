use std::ops::Range;

use super::{
    binding::{self, Buffer, BufferGroup, TextureBinding},
    frame::Framebuffer,
    model::{Material, Mesh, Model},
};

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );

    fn draw_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );
    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b Model,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );

    fn bind_material(&mut self, index: u32, material: &'b Material);
}

pub trait DrawLight<'a, 'b>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );
    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b binding::BufferGroup,
        light: &'b binding::BufferGroup,
    );
}

pub trait Binding<'a, 'b>
where
    'b: 'a,
{
    fn bind_textures(&mut self, index: u32, textures: &'b TextureBinding);
    fn bind_group(&mut self, index: u32, group: &'b BufferGroup);
    fn bind_buffer(&mut self, slot: u32, buffer: &'b Buffer);
    fn bind_vertex_buffer(&mut self, slot: u32, buffer: &'b Buffer);
    fn bind_index_buffer(&mut self, buffer: &'b Buffer);
}

pub trait DrawFramebuffer<'a, 'b>
where
    'b: 'a,
{
    fn draw_framebuffer(&mut self, frame: &'b Framebuffer);
}

use binding::BufferUsage;
use log::info;

use super::{
    binding, state, texture,
    traits::{Binding, DrawFramebuffer, Vertex},
};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FrameVertex {
    position: nalgebra::Vector3<f32>,
    tex_coords: nalgebra::Vector2<f32>,
}

unsafe impl bytemuck::Pod for FrameVertex {}
unsafe impl bytemuck::Zeroable for FrameVertex {}

impl Vertex for FrameVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<FrameVertex>() as wgpu::BufferAddress,
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
            ],
        }
    }
}

pub struct Framebuffer {
    pub vertex_buffer: binding::Buffer,
    pub textures: binding::TextureBinding,
    pub index_buffer: binding::Buffer,
}

impl Framebuffer {
    pub fn new<T: Into<Option<&'a str>>>(
        state: &state::WgpuState,
        label: T,
        layout: &wgpu::BindGroupLayout,
        textures: &[&texture::Texture],
    ) -> Self {
        let label = label.into();
        info!("Create framebuffer {:?}", &label.unwrap_or(""));
        Self {
            vertex_buffer: binding::Buffer::new_init(
                state,
                label,
                &[
                    FrameVertex {
                        position: [0.0, 0.0, 0.0].into(),
                        tex_coords: [0.0, 1.0].into(),
                    },
                    FrameVertex {
                        position: [1.0, 0.0, 0.0].into(),
                        tex_coords: [1.0, 1.0].into(),
                    },
                    FrameVertex {
                        position: [1.0, 1.0, 0.0].into(),
                        tex_coords: [1.0, 0.0].into(),
                    },
                    FrameVertex {
                        position: [0.0, 1.0, 0.0].into(),
                        tex_coords: [0.0, 0.0].into(),
                    },
                ],
                BufferUsage::Vertex,
            ),
            index_buffer: binding::Buffer::new_init(
                state,
                label,
                &[0, 1, 2, 0, 2, 3],
                BufferUsage::Index,
            ),
            textures: binding::TextureBinding::new_ref(state, label, layout, textures),
        }
    }

    pub fn update_textures(
        &mut self,
        state: &state::WgpuState,
        layout: &wgpu::BindGroupLayout,
        textures: &[&texture::Texture],
    ) {
        self.textures = binding::TextureBinding::new_ref(state, None, layout, textures)
    }
}

impl<'a, 'b> DrawFramebuffer<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_framebuffer(&mut self, frame: &'b Framebuffer, uniforms: &'b binding::BufferGroup) {
        self.bind_vertex_buffer(0, &frame.vertex_buffer);
        self.bind_index_buffer(&frame.index_buffer);
        self.bind_textures(0, &frame.textures);
        self.bind_group(1, &uniforms);
        self.draw_indexed(0..6, 0, 0..1);
    }
}

use log::info;
use nalgebra::{zero, Vector2, Vector3, Vector4};

use super::{
    binding::{self, BufferUsage},
    state,
    traits::{Binding, DrawGrid, Vertex},
};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GridVertex {
    position: Vector3<f32>,
    tex_coords: Vector2<f32>,
}

unsafe impl bytemuck::Pod for GridVertex {}
unsafe impl bytemuck::Zeroable for GridVertex {}

impl Vertex for GridVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<GridVertex>() as wgpu::BufferAddress,
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

pub const SIZE: f32 = 1024.0;
pub const TEX_COORD: f32 = 1024.0;

#[repr(C)]
#[derive(Copy, Clone)]
struct GridData {
    size: f32,
    _padding: [u32; 3],
    color: Vector4<f32>,
}

unsafe impl bytemuck::Pod for GridData {}
unsafe impl bytemuck::Zeroable for GridData {}

pub struct Grid {
    pub vertex_buffer: binding::Buffer,
    pub index_buffer: binding::Buffer,
    pub grid_group: binding::BufferGroup,
}

impl Grid {
    pub fn new<T: Into<Option<&'a str>>>(
        state: &state::WgpuState,
        label: T,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let label = label.into();
        info!("Create grid {:?}", &label.unwrap_or(""));
        Self {
            vertex_buffer: binding::Buffer::new_init(
                state,
                label,
                &[
                    GridVertex {
                        position: Vector3::new(-SIZE, 0.0, -SIZE),
                        tex_coords: zero(),
                    },
                    GridVertex {
                        position: Vector3::new(SIZE, 0.0, -SIZE),
                        tex_coords: Vector2::new(TEX_COORD, 0.0),
                    },
                    GridVertex {
                        position: Vector3::new(-SIZE, 0.0, SIZE),
                        tex_coords: Vector2::new(0.0, TEX_COORD),
                    },
                    GridVertex {
                        position: Vector3::new(SIZE, 0.0, SIZE),
                        tex_coords: Vector2::new(TEX_COORD, TEX_COORD),
                    },
                ],
                BufferUsage::Vertex,
            ),
            index_buffer: binding::Buffer::new_init(
                state,
                label,
                &[0, 1, 2, 1, 3, 2],
                BufferUsage::Index,
            ),
            grid_group: binding::BufferGroup::from_buffer(
                &state,
                "grid",
                &layout,
                &[&binding::Buffer::new_init(
                    &state,
                    "grid",
                    &[GridData {
                        size: 1.0,
                        _padding: [0u32; 3],
                        color: Vector4::new(1.0, 1.0, 1.0, 1.0),
                    }],
                    binding::BufferUsage::Uniform,
                )],
            ),
        }
    }
}

impl<'a, 'b> DrawGrid<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_grid(&mut self, grid: &'b Grid, uniforms: &'b binding::BufferGroup) {
        self.bind_vertex_buffer(0, &grid.vertex_buffer);
        self.bind_index_buffer(&grid.index_buffer);
        self.bind_group(0, uniforms);
        self.bind_group(1, &grid.grid_group);
        self.draw_indexed(0..6, 0, 0..1);
    }
}

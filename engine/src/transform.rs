use cgmath::{vec3, Deg, Matrix4, One, Vector3, Zero};

use crate::render::{binding, state};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl InstanceRaw {
    pub fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}

struct TransformRaw {
    pub position: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub rotation: Vector3<Deg<f32>>,
}

impl TransformRaw {
    pub fn new() -> Self {
        Self {
            position: Vector3::zero(),
            scale: vec3(f32::one(), f32::one(), f32::one()),
            rotation: vec3(Deg(f32::zero()), Deg(f32::zero()), Deg(f32::zero())),
        }
    }

    fn matrix(&self) -> Matrix4<f32> {
        return Matrix4::from_translation(self.position)
            * Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z)
            * Matrix4::from_angle_x(self.rotation.x)
            * Matrix4::from_angle_y(self.rotation.y)
            * Matrix4::from_angle_z(self.rotation.z);
    }

    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: self.matrix().into(),
        }
    }
}

pub struct Transform {
    transform: TransformRaw,
    pub buffer: binding::Buffer,
    dirty: bool,
}

impl Transform {
    pub fn new<L: Into<Option<&'a str>>>(state: &state::WgpuState, label: L) -> Self {
        let transform = TransformRaw::new();
        let buffer = binding::Buffer::new_init(
            state,
            label,
            &[transform.to_raw()],
            binding::BufferUsage::Vertex,
        );
        Self {
            transform,
            buffer,
            dirty: false,
        }
    }
}

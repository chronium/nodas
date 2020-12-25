use nalgebra::{Isometry3, Matrix4, Translation3, UnitQuaternion, Vector3};

use crate::{
    inspect::{InspectTransform, IntoInspect},
    render::{binding, state},
};

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

pub struct Transform {
    transform: Isometry3<f32>,
    scale: Vector3<f32>,
    buffer: binding::Buffer,
    pub dirty: bool,
}

impl IntoInspect for crate::transform::Transform {
    type Output = InspectTransform;

    fn into_inspect(&self) -> Self::Output {
        Self::Output::new(self.transform, self.scale.into())
    }
}

impl Transform {
    pub fn new<L: Into<Option<&'a str>>>(state: &state::WgpuState, label: L) -> Self {
        let transform = Isometry3::identity();
        let scale = Vector3::new(1.0, 1.0, 1.0);
        let buffer = binding::Buffer::new_init(
            state,
            label,
            &[InstanceRaw {
                model: (Matrix4::new_nonuniform_scaling(&scale) * transform.to_matrix()).into(),
            }],
            binding::BufferUsage::Transform,
        );
        Self {
            transform,
            scale,
            buffer,
            dirty: false,
        }
    }

    pub fn isometry(&self) -> Isometry3<f32> {
        self.transform
    }

    pub fn scale(&self) -> Vector3<f32> {
        self.scale
    }

    pub fn set_position(&mut self, position: Translation3<f32>) -> &mut Self {
        self.dirty = true;
        self.transform.translation = position;
        self
    }

    pub fn set_rotation(&mut self, rotation: UnitQuaternion<f32>) -> &mut Self {
        self.dirty = true;
        self.transform.rotation = rotation;
        self
    }

    pub fn set_scale(&mut self, scale: Vector3<f32>) -> &mut Self {
        self.dirty = true;
        self.scale = scale;
        self
    }

    pub fn buffer(&mut self, state: &state::WgpuState) -> &binding::Buffer {
        if self.dirty {
            self.dirty = false;
            self.buffer.write(
                state,
                &[InstanceRaw {
                    model: self.matrix().into(),
                }],
            );
        }
        &self.buffer
    }

    #[inline]
    fn matrix(&self) -> Matrix4<f32> {
        Matrix4::new_nonuniform_scaling(&self.scale) * self.transform.to_matrix()
    }
}

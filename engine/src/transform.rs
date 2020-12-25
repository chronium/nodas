use nalgebra::{Isometry3, Matrix4, Rotation3, Translation3, UnitQuaternion, Vector3};

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
    translation: Translation3<f32>,
    rotation: Vector3<f32>,
    scale: Vector3<f32>,
    buffer: binding::Buffer,
    pub dirty: bool,
}

impl IntoInspect for crate::transform::Transform {
    type Output = InspectTransform;

    fn into_inspect(&self) -> Self::Output {
        Self::Output::new(self.translation, self.rotation, self.scale)
    }
}

impl Transform {
    pub fn new<L: Into<Option<&'a str>>>(state: &state::WgpuState, label: L) -> Self {
        let translation = Translation3::identity();
        let scale = Vector3::new(1.0, 1.0, 1.0);
        let buffer = binding::Buffer::new_init(
            state,
            label,
            &[InstanceRaw {
                model: (Matrix4::new_nonuniform_scaling(&scale)
                    * Isometry3::from_parts(
                        translation,
                        UnitQuaternion::from_rotation_matrix(&Rotation3::identity()),
                    )
                    .to_matrix())
                .into(),
            }],
            binding::BufferUsage::Transform,
        );
        Self {
            translation,
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale,
            buffer,
            dirty: false,
        }
    }

    pub fn isometry(&self) -> Isometry3<f32> {
        Isometry3::from_parts(
            self.translation,
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.rotation.x)
                * UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.rotation.y)
                * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), self.rotation.z),
        )
    }

    pub fn scale(&self) -> Vector3<f32> {
        self.scale
    }

    pub fn set_position(&mut self, position: Translation3<f32>) -> &mut Self {
        self.dirty = true;
        self.translation = position;
        self
    }

    pub fn set_rotation(&mut self, rotation: Vector3<f32>) -> &mut Self {
        self.dirty = true;
        self.rotation = rotation;
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
        self.isometry().to_matrix() * Matrix4::new_nonuniform_scaling(&self.scale)
    }
}

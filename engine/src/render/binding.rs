use log::info;
use wgpu::util::DeviceExt;

use super::{state, texture, traits::Binding};

#[derive(Debug)]
pub enum BufferUsage {
    Vertex,
    Uniform,
    Index,
    Transform,
}

impl From<BufferUsage> for wgpu::BufferUsage {
    fn from(buf: BufferUsage) -> Self {
        match buf {
            BufferUsage::Vertex => wgpu::BufferUsage::VERTEX,
            BufferUsage::Uniform => wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            BufferUsage::Index => wgpu::BufferUsage::INDEX,
            BufferUsage::Transform => wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        }
    }
}

pub struct Buffer {
    pub buffer: wgpu::Buffer,
}

impl Buffer {
    pub fn new_init<A: bytemuck::Pod, L: Into<Option<&'a str>>>(
        state: &state::WgpuState,
        label: L,
        data: &[A],
        usage: BufferUsage,
    ) -> Self {
        let label = label.into();
        info!("Init {:?} buffer {:?}", &usage, &label.unwrap_or(""));
        Self {
            buffer: state
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: label,
                    usage: usage.into(),
                    contents: bytemuck::cast_slice(&data),
                }),
        }
    }

    pub fn write<A: bytemuck::Pod>(&self, state: &state::WgpuState, data: &[A]) {
        state.write_buffer(&self.buffer, data);
    }
}

impl From<&&'a Buffer> for wgpu::BindingResource<'a> {
    fn from(buf: &&'a Buffer) -> Self {
        wgpu::BindingResource::Buffer(buf.buffer.slice(..))
    }
}

pub struct BufferGroup {
    pub bind_group: wgpu::BindGroup,
    pub label: String,
}

impl BufferGroup {
    pub fn from_buffer<T: Into<Option<&'a str>>>(
        state: &state::WgpuState,
        label: T,
        layout: &wgpu::BindGroupLayout,
        group: &[&Buffer],
    ) -> Self {
        let label = label.into();
        info!("Create buffer group {:?}", &label.unwrap_or(""));
        Self {
            bind_group: state
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: label,
                    layout,
                    entries: group
                        .iter()
                        .enumerate()
                        .map(|(i, buf)| wgpu::BindGroupEntry {
                            binding: i as u32,
                            resource: buf.into(),
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                }),
            label: String::from(label.unwrap_or("")),
        }
    }
}

pub struct TextureBinding {
    pub bind_group: wgpu::BindGroup,
    pub label: String,
}

impl TextureBinding {
    pub fn new<T: Into<Option<&'a str>>>(
        state: &state::WgpuState,
        label: T,
        layout: &wgpu::BindGroupLayout,
        textures: &[texture::Texture],
    ) -> Self {
        let label = label.into();
        Self {
            bind_group: state
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label,
                    layout,
                    entries: textures
                        .iter()
                        .enumerate()
                        .flat_map(|(i, tex)| {
                            vec![
                                wgpu::BindGroupEntry {
                                    binding: (i * 2) as u32,
                                    resource: wgpu::BindingResource::TextureView(&tex.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: (i * 2 + 1) as u32,
                                    resource: wgpu::BindingResource::Sampler(&tex.sampler),
                                },
                            ]
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                }),
            label: String::from(label.unwrap_or("")),
        }
    }

    pub fn new_ref<T: Into<Option<&'a str>>>(
        state: &state::WgpuState,
        label: T,
        layout: &wgpu::BindGroupLayout,
        textures: &[&texture::Texture],
    ) -> Self {
        let label: Option<&str> = label.into();
        Self {
            bind_group: state
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label,
                    layout,
                    entries: textures
                        .iter()
                        .enumerate()
                        .flat_map(|(i, tex)| {
                            vec![
                                wgpu::BindGroupEntry {
                                    binding: (i * 2) as u32,
                                    resource: wgpu::BindingResource::TextureView(&tex.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: (i * 2 + 1) as u32,
                                    resource: wgpu::BindingResource::Sampler(&tex.sampler),
                                },
                            ]
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                }),
            label: String::from(label.unwrap_or("")),
        }
    }
}

impl<'a, 'b> Binding<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn bind_textures(&mut self, index: u32, textures: &'b TextureBinding) {
        self.set_bind_group(index, &textures.bind_group, &[]);
    }

    fn bind_group(&mut self, index: u32, group: &'b BufferGroup) {
        self.set_bind_group(index, &group.bind_group, &[]);
    }

    fn bind_buffer(&mut self, slot: u32, buffer: &'b Buffer) {
        self.set_vertex_buffer(slot, buffer.buffer.slice(..));
    }

    fn bind_vertex_buffer(&mut self, slot: u32, buffer: &'b Buffer) {
        self.set_vertex_buffer(slot, buffer.buffer.slice(..));
    }

    fn bind_index_buffer(&mut self, buffer: &'b Buffer) {
        self.set_index_buffer(buffer.buffer.slice(..));
    }
}

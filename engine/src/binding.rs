use crate::{state, texture};

pub enum BufferType<'a> {
    Buffer(&'a wgpu::Buffer),
}

impl From<&'_ BufferType<'a>> for wgpu::BindingResource<'a> {
    fn from(buf: &BufferType<'a>) -> Self {
        match buf {
            BufferType::Buffer(ref b) => wgpu::BindingResource::Buffer(b.slice(..)),
        }
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
        group: &[BufferType],
    ) -> Self {
        let label: Option<&str> = label.into();
        Self {
            bind_group: state
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: label.into(),
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

pub trait Binding<'a, 'b>
where
    'b: 'a,
{
    fn bind_textures(&mut self, index: u32, textures: &'b TextureBinding);
    fn bind_group(&mut self, index: u32, group: &'b BufferGroup);
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
}

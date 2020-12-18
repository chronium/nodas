pub mod binding;
pub mod frame;
pub mod model;
pub mod renderpass;
pub mod state;
pub mod texture;
pub mod traits;

pub struct Layouts {
    pub material: wgpu::BindGroupLayout,
    pub uniforms: wgpu::BindGroupLayout,
    pub light: wgpu::BindGroupLayout,
    pub frame: wgpu::BindGroupLayout,
}

pub struct Pipelines {
    pub forward: wgpu::RenderPipeline,
    pub light: wgpu::RenderPipeline,
    pub depth: wgpu::RenderPipeline,
}

pub fn frame_layout(state: &state::WgpuState) -> wgpu::BindGroupLayout {
    state.create_layout(
        "framebuffer",
        &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::SampledTexture {
                    multisampled: false,
                    dimension: wgpu::TextureViewDimension::D2,
                    component_type: wgpu::TextureComponentType::Float,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Sampler { comparison: true },
                count: None,
            },
        ],
    )
}

pub fn material_layout(state: &state::WgpuState) -> wgpu::BindGroupLayout {
    state.create_layout(
        "material",
        &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::SampledTexture {
                    multisampled: true,
                    dimension: wgpu::TextureViewDimension::D2,
                    component_type: wgpu::TextureComponentType::Uint,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Sampler { comparison: false },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::SampledTexture {
                    multisampled: true,
                    dimension: wgpu::TextureViewDimension::D2,
                    component_type: wgpu::TextureComponentType::Float,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Sampler { comparison: false },
                count: None,
            },
        ],
    )
}

pub fn uniforms_layout(state: &state::WgpuState) -> wgpu::BindGroupLayout {
    state.create_layout(
        "uniforms",
        &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: None,
            },
            count: None,
        }],
    )
}

pub fn light_layout(state: &state::WgpuState) -> wgpu::BindGroupLayout {
    state.create_layout(
        "light",
        &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: None,
            },
            count: None,
        }],
    )
}

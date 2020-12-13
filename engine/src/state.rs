use std::{fs, path::Path};

use anyhow::*;

use wgpu_mipmap::{MipmapGenerator, RecommendedMipmapGenerator};
use winit::window::Window;

pub struct WgpuState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain_descriptor: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    mipgen: Box<dyn MipmapGenerator>,
}

impl WgpuState {
    pub async fn new(window: &Window, present_format: wgpu::TextureFormat) -> Result<Self> {
        let size = window.inner_size().clone();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .context("Could not request adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .context("Could not request device and queue")?;

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: present_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        let mipgen = Box::new(RecommendedMipmapGenerator::new(&device));

        Ok(Self {
            surface,
            device,
            queue,
            swap_chain_descriptor,
            swap_chain,
            mipgen,
        })
    }

    pub fn create_layout<T: Into<Option<&'a str>>>(
        &self,
        name: T,
        entries: &[wgpu::BindGroupLayoutEntry],
    ) -> wgpu::BindGroupLayout {
        self.device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries,
                label: name.into(),
            })
    }

    pub fn create_pipeline_layout<T: Into<Option<&'a str>>>(
        &self,
        name: T,
        bindings: &[&wgpu::BindGroupLayout],
    ) -> Result<wgpu::PipelineLayout> {
        Ok(self
            .device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: name.into(),
                bind_group_layouts: bindings,
                push_constant_ranges: &[],
            }))
    }

    pub fn create_render_pipeline<
        P: AsRef<Path>,
        D: Into<Option<wgpu::TextureFormat>>,
        T: Into<Option<&'a str>>,
    >(
        &self,
        layout: &wgpu::PipelineLayout,
        pipeline: T,
        color_format: wgpu::TextureFormat,
        color_blend: wgpu::BlendDescriptor,
        alpha_blend: wgpu::BlendDescriptor,
        depth_format: D,
        vertex_descs: &[wgpu::VertexBufferDescriptor],
        vertex_shader: P,
        fragment_shader: P,
    ) -> Result<wgpu::RenderPipeline> {
        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let vs_module = self.device().create_shader_module(wgpu::util::make_spirv(
            fs::read(res_dir.join(vertex_shader.as_ref()))
                .context(format!(
                    "Could not read shader {:?}",
                    vertex_shader.as_ref()
                ))?
                .as_slice(),
        ));
        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let fs_module = self.device().create_shader_module(wgpu::util::make_spirv(
            fs::read(res_dir.join(fragment_shader.as_ref()))
                .context(format!(
                    "Could not read shader {:?}",
                    fragment_shader.as_ref()
                ))?
                .as_slice(),
        ));

        Ok(self
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: pipeline.into(),
                layout: Some(layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::Back,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                    clamp_depth: false,
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: color_format,
                    color_blend,
                    alpha_blend,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                depth_stencil_state: depth_format.into().map(|format| {
                    wgpu::DepthStencilStateDescriptor {
                        format,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilStateDescriptor::default(),
                    }
                }),
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint32,
                    vertex_buffers: vertex_descs,
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            }))
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }

    pub fn recreate_swapchain(&mut self, width: u32, height: u32) {
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    }

    pub fn frame(&mut self) -> Result<wgpu::SwapChainFrame, wgpu::SwapChainError> {
        self.swap_chain.get_current_frame()
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn mipgen(&self) -> &dyn MipmapGenerator {
        &*self.mipgen
    }

    pub fn width(&self) -> u32 {
        self.swap_chain_descriptor.width
    }

    pub fn height(&self) -> u32 {
        self.swap_chain_descriptor.height
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.swap_chain_descriptor.format
    }
}

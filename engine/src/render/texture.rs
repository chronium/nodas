use std::{num::NonZeroU8, path::Path};

use anyhow::*;
use image::GenericImageView;
use log::info;

use super::state;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    #[allow(dead_code)]
    pub fn from_bytes(
        state: &state::WgpuState,
        bytes: &[u8],
        label: &str,
        is_normal_map: bool,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(state, &img, Some(label), is_normal_map)
    }

    pub fn from_image(
        state: &state::WgpuState,
        img: &image::DynamicImage,
        label: Option<&str>,
        is_normal_map: bool,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth: 1,
        };

        let descriptor = wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 12,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if is_normal_map {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            },
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        };

        let texture = state.device().create_texture(&descriptor);

        state.queue().write_texture(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * dimensions.0,
                rows_per_image: dimensions.1,
            },
            size,
        );

        let mut encoder = state.device().create_command_encoder(&Default::default());
        state
            .mipgen()
            .generate(state.device(), &mut encoder, &texture, &descriptor)
            .context(format!(
                "Could not generate mipmap for {}",
                label.unwrap_or("unknown")
            ))?;
        state.queue().submit(std::iter::once(encoder.finish()));

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = state.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            anisotropy_clamp: NonZeroU8::new(16),
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub fn create_depth_texture(state: &state::WgpuState, label: &str) -> Self {
        let size = wgpu::Extent3d {
            width: state.width(),
            height: state.height(),
            depth: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        };
        let texture = state.device().create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = state.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });
        info!("Create depth texture {:?}", label);

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn load<P: AsRef<Path>>(
        state: &state::WgpuState,
        path: P,
        is_normal_map: bool,
    ) -> Result<Self> {
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();
        info!("Load texture {:?}", path_copy);

        let img = image::open(path)?;
        Self::from_image(state, &img, label, is_normal_map)
    }
}

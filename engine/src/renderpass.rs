pub type ColorAttachment<'a> = (&'a wgpu::TextureView, wgpu::LoadOp<wgpu::Color>);
pub type DepthAttachment<'a> = (&'a wgpu::TextureView, wgpu::LoadOp<f32>);

pub trait IntoColorAttachment<'a> {
    fn color_attachment(&self) -> wgpu::RenderPassColorAttachmentDescriptor<'a>;
}

impl<'a> IntoColorAttachment<'a> for ColorAttachment<'a> {
    fn color_attachment(&self) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
        wgpu::RenderPassColorAttachmentDescriptor {
            attachment: &self.0,
            resolve_target: None,
            ops: wgpu::Operations {
                load: self.1,
                store: true,
            },
        }
    }
}

pub trait IntoDepthAttachment<'a> {
    fn depth_attachment(&self) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<'a>;
}

impl<'a> IntoDepthAttachment<'a> for DepthAttachment<'a> {
    fn depth_attachment(&self) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<'a> {
        wgpu::RenderPassDepthStencilAttachmentDescriptor {
            attachment: &self.0,
            depth_ops: Some(wgpu::Operations {
                load: self.1,
                store: true,
            }),
            stencil_ops: None,
        }
    }
}

pub fn render_pass<'a, D>(
    encoder: &'a mut wgpu::CommandEncoder,
    color_attachments: &'a [&dyn IntoColorAttachment<'a>],
    depth_attachment: D,
) -> wgpu::RenderPass<'a>
where
    D: Into<Option<&'a dyn IntoDepthAttachment<'a>>>,
{
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: color_attachments
            .iter()
            .map(|col| col.color_attachment())
            .collect::<Vec<_>>()
            .as_ref(),
        depth_stencil_attachment: depth_attachment
            .into()
            .map(|depth| depth.depth_attachment()),
    })
}

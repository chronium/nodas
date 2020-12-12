#![feature(in_band_lifetimes)]

mod binding;
mod camera;
mod model;
mod render;
mod state;
mod texture;

use futures::executor::block_on;

use imgui::{im_str, Condition, FontSource};
use model::Vertex;
use model::{DrawLight, DrawModel};
use winit::{
    dpi::LogicalPosition,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::util::DeviceExt;

use cgmath::{InnerSpace, Rotation3, Zero};

use anyhow::*;

#[repr(C)]
#[derive(Copy, Clone)]
struct Light {
    position: cgmath::Vector3<f32>,
    _padding: u32,
    color: cgmath::Vector3<f32>,
}

unsafe impl bytemuck::Pod for Light {}
unsafe impl bytemuck::Zeroable for Light {}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}
#[repr(C)]
#[derive(Copy, Clone)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
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

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniforms {
    view_position: cgmath::Vector4<f32>,
    view_proj: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: Zero::zero(),
            view_proj: cgmath::Matrix4::identity(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous();
        self.view_proj = projection.calc_matrix() * camera.calc_matrix()
    }
}

const NUM_INSTANCES_PER_ROW: u32 = 10;

struct Engine {
    window: Window,
    state: state::WgpuState,
    pipelines: render::Pipelines,
    camera: camera::Camera,
    projection: camera::Projection,
    camera_controller: camera::CameraController,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_group: binding::BufferGroup,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    obj_model: model::Model,
    light_buffer: wgpu::Buffer,
    light_group: binding::BufferGroup,
    light: Light,
    debug_material: model::Material,
    last_mouse_pos: LogicalPosition<f64>,
    current_mouse_pos: LogicalPosition<f64>,
    mouse_pressed: bool,
    imgui: imgui::Context,
    imgui_renderer: imgui_wgpu::Renderer,
    last_cursor: Option<imgui::MouseCursor>,
    platform: imgui_winit_support::WinitPlatform,
}

impl Engine {
    async fn new(window: Window) -> Result<Self> {
        let state = state::WgpuState::new(&window, wgpu::TextureFormat::Bgra8UnormSrgb)
            .await
            .unwrap();

        let layouts = render::Layouts {
            material: render::material_layout(&state),
            uniforms: render::uniforms_layout(&state),
            light: render::light_layout(&state),
        };

        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            camera::Projection::new(state.width(), state.height(), cgmath::Deg(75.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, 350.0);

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera, &projection);

        let uniform_buffer = state
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });

        let uniform_group = binding::BufferGroup::from_buffer(
            &state,
            "uniforms",
            &layouts.uniforms,
            &[binding::BufferType::Buffer(&uniform_buffer)],
        );

        let light = Light {
            position: (2.0, 2.0, 2.0).into(),
            _padding: 0,
            color: (1.0, 1.0, 1.0).into(),
        };

        let light_buffer = state
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });

        let light_group = binding::BufferGroup::from_buffer(
            &state,
            "light",
            &layouts.light,
            &[binding::BufferType::Buffer(&light_buffer)],
        );

        let depth_texture = texture::Texture::create_depth_texture(&state, "depth_texture");

        let forward_layout = state.create_pipeline_layout(
            None,
            &[&layouts.material, &layouts.uniforms, &layouts.light],
        )?;

        let light_layout =
            state.create_pipeline_layout(None, &[&layouts.uniforms, &layouts.light])?;

        let forward = state.create_render_pipeline(
            &forward_layout,
            None,
            state.format(),
            texture::Texture::DEPTH_FORMAT,
            &[model::ModelVertex::desc(), InstanceRaw::desc()],
            "shader.vert.spv",
            "shader.frag.spv",
        )?;

        let light_pipeline = state.create_render_pipeline(
            &light_layout,
            None,
            state.format(),
            texture::Texture::DEPTH_FORMAT,
            &[model::ModelVertex::desc()],
            "light.vert.spv",
            "light.frag.spv",
        )?;

        let pipelines = render::Pipelines {
            forward,
            light: light_pipeline,
        };

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let obj_model = model::Model::load(&state, &layouts.material, res_dir.join("cube.obj"))?;

        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = cgmath::Vector3 { x, y: 0.0, z };

                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(
                            position.clone().normalize(),
                            cgmath::Deg(45.0),
                        )
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer =
            state
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: bytemuck::cast_slice(&instance_data),
                    usage: wgpu::BufferUsage::VERTEX,
                });

        let debug_material = {
            let diffuse_path = res_dir.join("cobblestone_floor_08_diff_4k.jpg");
            let diffuse_texture = texture::Texture::load(&state, diffuse_path, false).unwrap();

            let normal_path = res_dir.join("cobblestone_floor_08_nor_4k.jpg");
            let normal_texture = texture::Texture::load(&state, normal_path, true).unwrap();

            model::Material::new(
                &state,
                "alt-material",
                diffuse_texture,
                normal_texture,
                &layouts.material,
            )
        };

        let mut imgui = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui.set_ini_filename(None);

        let hidpi_factor = window.scale_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        let imgui_renderer = imgui_wgpu::Renderer::new(
            &mut imgui,
            &state.device(),
            &state.queue(),
            imgui_wgpu::RendererConfig {
                texture_format: state.format(),
                ..Default::default()
            },
        );

        Ok(Self {
            window,
            state,
            pipelines,
            camera,
            projection,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_group,
            instances,
            instance_buffer,
            depth_texture,
            obj_model,
            light_buffer,
            light_group,
            light,
            debug_material,
            last_mouse_pos: (0.0, 0.0).into(),
            current_mouse_pos: (0.0, 0.0).into(),
            mouse_pressed: false,
            imgui,
            imgui_renderer,
            last_cursor: None,
            platform,
        })
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::<u32> {
            width: self.state.width(),
            height: self.state.height(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.projection.resize(new_size.width, new_size.height);
        self.state
            .recreate_swapchain(new_size.width, new_size.height);
        self.depth_texture = texture::Texture::create_depth_texture(&self.state, "depth_texture");
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.current_mouse_pos = LogicalPosition {
                    x: position.to_logical::<f64>(self.state.width() as f64).x,
                    y: position.to_logical::<f64>(self.state.height() as f64).y,
                };
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: std::time::Duration) {
        let old_position: cgmath::Vector3<_> = self.light.position.into();
        self.light.position = cgmath::Quaternion::from_axis_angle(
            (0.0, 1.0, 0.0).into(),
            cgmath::Deg(60.0 * dt.as_secs_f32()),
        ) * old_position;
        self.state
            .queue()
            .write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light]));
        self.imgui.io_mut().update_delta_time(dt);
    }

    fn render(&mut self, dt: std::time::Duration) -> Result<(), wgpu::SwapChainError> {
        let mut encoder =
            self.state
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let sc = self.state.frame()?.output;

        let ui = self.imgui.frame();
        {
            let window = imgui::Window::new(im_str!("Hello Imgui from WGPU!"));
            window
                .size([300.0, 100.0], Condition::FirstUseEver)
                .build(&ui, || {
                    ui.text(im_str!("Hello world!"));
                    ui.text(im_str!("This is a demo of imgui-rs using imgui-wgpu!"));
                    ui.separator();
                    let mouse_pos = ui.io().mouse_pos;
                    ui.text(im_str!(
                        "Mouse Position: ({:.1}, {:.1})",
                        mouse_pos[0],
                        mouse_pos[1],
                    ));
                });
        }

        if self.mouse_pressed && !ui.is_any_item_active() {
            let mouse_dx = self.current_mouse_pos.x - self.last_mouse_pos.x;
            let mouse_dy = self.current_mouse_pos.y - self.last_mouse_pos.y;
            self.camera_controller.process_mouse(mouse_dx, mouse_dy);
        }
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.last_mouse_pos = self.current_mouse_pos;
        self.uniforms
            .update_view_proj(&self.camera, &self.projection);
        self.state.queue().write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &sc.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.pipelines.forward);

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw_model_instanced_with_material(
                &self.obj_model,
                &self.debug_material,
                0..self.instances.len() as u32,
                &self.uniform_group,
                &self.light_group,
            );

            render_pass.set_pipeline(&self.pipelines.light);
            render_pass.draw_light_model(&self.obj_model, &self.uniform_group, &self.light_group);
        }

        {
            if self.last_cursor != ui.mouse_cursor() {
                self.last_cursor = ui.mouse_cursor();
                self.platform.prepare_render(&ui, &self.window);
            }

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &sc.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            self.imgui_renderer
                .render(
                    ui.render(),
                    &self.state.queue(),
                    &self.state.device(),
                    &mut pass,
                )
                .expect("Failed to render UI!");
        }

        self.state.queue().submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn window_id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    pub fn inmgui_event<T>(&mut self, event: &Event<T>) {
        self.platform
            .handle_event(self.imgui.io_mut(), &self.window, event)
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("WGPU World!")
        .build(&event_loop)
        .unwrap();

    let mut engine = block_on(Engine::new(window)).unwrap();
    let mut last_render_time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                engine.set_title(format!("{}", (1.0 / dt.as_secs_f32()) as u32).as_str());

                engine.update(dt);
                match engine.render(dt) {
                    Ok(_) => {}
                    Err(wgpu::SwapChainError::Lost) => engine.resize(engine.size()),
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                engine.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == engine.window_id() => {
                if !engine.input(event) {
                    match event {
                        WindowEvent::Resized(physical_size) => {
                            engine.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            engine.resize(**new_inner_size);
                        }
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => *control_flow = ControlFlow::Exit,
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        engine.inmgui_event(&event);
    });
}

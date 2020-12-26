#![feature(in_band_lifetimes, cell_update)]

mod camera;
mod inspect;
mod render;
mod transform;
mod world;

use futures::executor::block_on;

use imgui::{im_str, ComboBox, Condition, FontSource, ImString};
use imgui_inspect::{InspectArgsStruct, InspectRenderStruct};
use inspect::IntoInspect;
use log::info;
use nalgebra::Matrix4;
use render::{
    binding, frame, model, renderpass, state, texture,
    traits::{DrawFramebuffer, DrawGrid, DrawLight, Vertex},
};
use winit::{
    dpi::LogicalPosition,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use anyhow::*;

#[repr(C)]
#[derive(Copy, Clone)]
struct Light {
    position: nalgebra::Vector3<f32>,
    ty: f32,
    color: nalgebra::Vector3<f32>,
}

unsafe impl bytemuck::Pod for Light {}
unsafe impl bytemuck::Zeroable for Light {}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniforms {
    view_position: nalgebra::Vector4<f32>,
    view_proj: nalgebra::Matrix4<f32>,
    view: nalgebra::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_position: nalgebra::zero(),
            view_proj: nalgebra::Matrix4::identity(),
            view: nalgebra::Matrix4::identity(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera) {
        self.view_position = camera.eye.to_homogeneous();
        self.view_proj = camera.view_proj;
        self.view = camera.view_transform().to_homogeneous();
    }
}

struct Engine {
    window: Window,
    state: state::WgpuState,
    pipelines: render::Pipelines,
    camera: camera::Camera,
    camera_controller: camera::flycam::FlyCamController,
    uniforms: Uniforms,
    uniform_buffer: binding::Buffer,
    uniform_group: binding::BufferGroup,
    depth_texture: texture::Texture,
    obj_model: model::Model,
    light_buffer: binding::Buffer,
    light_group: binding::BufferGroup,
    light: Light,
    last_mouse_pos: LogicalPosition<f64>,
    current_mouse_pos: LogicalPosition<f64>,
    mouse_pressed: bool,
    imgui: imgui::Context,
    imgui_renderer: imgui_wgpu::Renderer,
    last_cursor: Option<imgui::MouseCursor>,
    platform: imgui_winit_support::WinitPlatform,
    light_depth_map: texture::Texture,
    framebuffer: frame::Framebuffer,
    layouts: render::Layouts,
    world: world::World,
    grid: render::grid::Grid,
}

impl Engine {
    async fn new(window: Window) -> Result<Self> {
        let state = state::WgpuState::new(&window, wgpu::TextureFormat::Bgra8UnormSrgb)
            .await
            .unwrap();
        info!("Wgpu initialized");

        let layouts = render::Layouts {
            material: render::material_layout(&state),
            uniforms: render::uniforms_layout(&state),
            light: render::light_layout(&state),
            frame: render::frame_layout(&state),
            grid: render::grid_layout(&state),
        };

        let camera = camera::Camera::new(
            [0.0, 5.0, 10.0].into(),
            [0.0, 0.0, 0.0].into(),
            camera::projection::Projection::new(state.width(), state.height(), 75.0, 0.1, 100.0),
        );
        let camera_controller = camera::flycam::FlyCamController::new(4.0, 100.0);
        info!("Camera and controller initialized");

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = binding::Buffer::new_init(
            &state,
            "uniforms",
            &[uniforms],
            binding::BufferUsage::Uniform,
        );

        let uniform_group = binding::BufferGroup::from_buffer(
            &state,
            "uniforms",
            &layouts.uniforms,
            &[&uniform_buffer],
        );

        let light = Light {
            position: [-0.25, 0.25, -0.25].into(),
            ty: 0.0,
            color: [1.0, 1.0, 1.0].into(),
        };

        let light_buffer =
            binding::Buffer::new_init(&state, "light", &[light], binding::BufferUsage::Uniform);

        let light_group =
            binding::BufferGroup::from_buffer(&state, "light", &layouts.light, &[&light_buffer]);

        let depth_texture = texture::Texture::create_depth_texture(&state, "depth_texture");

        let forward_layout = state.create_pipeline_layout(
            "forward",
            &[&layouts.material, &layouts.uniforms, &layouts.light],
        )?;

        let light_layout =
            state.create_pipeline_layout("light", &[&layouts.uniforms, &layouts.light])?;

        let depth_layout =
            state.create_pipeline_layout("depth", &[&layouts.frame, &layouts.uniforms])?;

        let grid_layout =
            state.create_pipeline_layout("grid", &[&layouts.uniforms, &layouts.grid])?;

        let forward = state.create_render_pipeline(
            &forward_layout,
            "forward_pipeline",
            state.format(),
            wgpu::BlendDescriptor::REPLACE,
            wgpu::BlendDescriptor::REPLACE,
            (texture::Texture::DEPTH_FORMAT, true),
            &[model::ModelVertex::desc(), transform::InstanceRaw::desc()],
            "shader.vert.spv",
            "shader.frag.spv",
            true,
        )?;

        let light_pipeline = state.create_render_pipeline(
            &light_layout,
            "light_pipeline",
            state.format(),
            wgpu::BlendDescriptor::REPLACE,
            wgpu::BlendDescriptor::REPLACE,
            (texture::Texture::DEPTH_FORMAT, true),
            &[model::ModelVertex::desc()],
            "light.vert.spv",
            "light.frag.spv",
            true,
        )?;

        let depth_pipeline = state.create_render_pipeline(
            &depth_layout,
            "depth_pipeline",
            state.format(),
            wgpu::BlendDescriptor::REPLACE,
            wgpu::BlendDescriptor::REPLACE,
            None,
            &[frame::FrameVertex::desc()],
            "depth_frame.vert.spv",
            "depth_frame.frag.spv",
            true,
        )?;

        let grid_pipeline = state.create_render_pipeline(
            &grid_layout,
            "grid_pipeline",
            state.format(),
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            (texture::Texture::DEPTH_FORMAT, false),
            &[render::grid::GridVertex::desc()],
            "grid.vert.spv",
            "grid.frag.spv",
            false,
        )?;

        let pipelines = render::Pipelines {
            forward,
            light: light_pipeline,
            depth: depth_pipeline,
            grid: grid_pipeline,
        };

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let obj_model = model::Model::load(&state, &layouts.material, res_dir.join("cube.obj"))?;

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

        let light_depth_map = texture::Texture::create_depth_texture(&state, "light_depth_map");

        let framebuffer = frame::Framebuffer::new(
            &state,
            "depth_framebuffer",
            &layouts.frame,
            &[&depth_texture],
        );

        let mut world = world::World::new();

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        world.load_model(&state, &layouts, "block", res_dir.join("cube.obj"))?;
        world.load_model(
            &state,
            &layouts,
            "pizza_box",
            res_dir.join("14037_Pizza_Box_v2_L1.obj"),
        )?;

        world.push_entity((
            world::ModelIdent("block".into()),
            transform::Transform::new(&state, "block_transform"),
        ))?;

        let mut transform = transform::Transform::new(&state, "block_transform");
        transform.set_position(nalgebra::Translation3::new(-2.5, 0.0, 0.0));
        world.push_entity((world::ModelIdent("block".into()), transform))?;

        world.update_collision_world();

        let grid = render::grid::Grid::new(&state, "grid", &layouts.grid);

        Ok(Self {
            window,
            state,
            pipelines,
            camera,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_group,
            depth_texture,
            obj_model,
            light_buffer,
            light_group,
            light,
            last_mouse_pos: (0.0, 0.0).into(),
            current_mouse_pos: (0.0, 0.0).into(),
            mouse_pressed: false,
            imgui,
            imgui_renderer,
            last_cursor: None,
            platform,
            light_depth_map,
            framebuffer,
            layouts,
            world,
            grid,
        })
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::<u32> {
            width: self.state.width(),
            height: self.state.height(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        info!(
            "Resize from {:?} to {:?}",
            (self.state.width() as u32, self.state.height() as u32),
            (new_size.width as u32, new_size.height as u32)
        );
        self.camera.resize(new_size.width, new_size.height);
        self.state
            .recreate_swapchain(new_size.width, new_size.height);
        self.depth_texture = texture::Texture::create_depth_texture(&self.state, "depth_texture");
        self.framebuffer
            .update_textures(&self.state, &self.layouts.frame, &[&self.depth_texture]);
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
            /*WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }*/
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
        /*let old_position: cgmath::Vector3<_> = self.light.position.into();
        self.light.position = cgmath::Quaternion::from_axis_angle(
            (0.0, 1.0, 0.0).into(),
            cgmath::Deg(60.0 * dt.as_secs_f32()),
        ) * old_position;
        self.light_buffer.write(&self.state, &[self.light]);*/
        self.imgui.io_mut().update_delta_time(dt);
        self.world.update_collision_world();
    }

    fn render(&mut self, dt: std::time::Duration) -> Result<(), wgpu::SwapChainError> {
        struct UIData<'a> {
            entry: Option<legion::world::Entry<'a>>,
            models: Vec<String>,
        }

        let mut encoder = self.state.encoder();

        let sc = self.state.frame()?.output;

        let raycast = self.world.raycast(&self.camera.ray(), 1024.0);

        let models = self
            .world
            .models
            .keys()
            .map(|m| m.0.clone())
            .collect::<Vec<_>>();

        let entry = if let Some(entity) = raycast {
            if let Some(entry) = self.world.entry(entity) {
                Some(entry)
            } else {
                None
            }
        } else {
            None
        };

        let ui_data = UIData { entry, models };

        let mut updated_transform = false;

        let ui = self.imgui.frame();
        {
            let window = imgui::Window::new(im_str!("Hello Imgui from WGPU!"));
            window
                .size(
                    [self.state.width() as f32, self.state.height() as f32],
                    Condition::Always,
                )
                .title_bar(false)
                .position([0.0, 0.0], Condition::Always)
                .draw_background(false)
                .menu_bar(false)
                .build(&ui, || {
                    ui.text(im_str!("FPS: {}", (1.0 / dt.as_secs_f32()).round()));
                    ui.separator();
                    let mouse_pos = ui.io().mouse_pos;
                    ui.text(im_str!(
                        "Mouse Position: ({:.1}, {:.1})",
                        mouse_pos[0],
                        mouse_pos[1],
                    ));

                    if let Some(mut entry) = ui_data.entry {
                        {
                            let transform = entry.get_component_mut::<transform::Transform>().ok();
                            if let Some(mut transform) = transform {
                                let mut inspect = transform.into_inspect();
                                let init_inspect = inspect.clone();
                                <inspect::InspectTransform as InspectRenderStruct<
                                    inspect::InspectTransform,
                                >>::render_mut(
                                    &mut [&mut inspect],
                                    "transform",
                                    &ui,
                                    &InspectArgsStruct::default(),
                                );

                                if inspect != init_inspect {
                                    transform
                                        .set_position(inspect.position())
                                        .set_rotation(inspect.rotation())
                                        .set_scale(inspect.scale());
                                    updated_transform = true;
                                    transform.dirty = true;
                                }
                            }
                        }
                        {
                            let model = entry.get_component_mut::<world::ModelIdent>().ok();
                            if let Some(mut model) = model {
                                let mut index = ui_data
                                    .models
                                    .iter()
                                    .enumerate()
                                    .find(|(_, m)| *m == &model.0)
                                    .map(|(i, _)| i)
                                    .expect("Must have model");
                                let init = index;
                                let imstrs = ui_data
                                    .models
                                    .iter()
                                    .map(|m| im_str!("{}", m))
                                    .collect::<Vec<_>>();
                                ComboBox::new(im_str!("model")).build_simple(
                                    &ui,
                                    &mut index,
                                    imstrs.as_slice(),
                                    &|s: &ImString| s.into(),
                                );

                                if init != index {
                                    model.0 = ui_data.models[index].clone();
                                    updated_transform = true;
                                }
                            }
                        }
                    }
                });
        }

        if updated_transform {
            self.world
                .update_entity_world_transform(raycast.unwrap())
                .expect("Internal err");
        }

        if self.mouse_pressed && !ui.is_any_item_hovered() {
            let mouse_dx = self.current_mouse_pos.x - self.last_mouse_pos.x;
            let mouse_dy = self.current_mouse_pos.y - self.last_mouse_pos.y;
            self.camera_controller.process_mouse(mouse_dx, mouse_dy);
        }
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.last_mouse_pos = self.current_mouse_pos;
        self.uniforms.update_view_proj(&self.camera);
        self.uniform_buffer.write(&self.state, &[self.uniforms]);

        {
            let color_attachments: &[&dyn renderpass::IntoColorAttachment] = &[&(
                &sc.view,
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
            )];

            let depth_attachment: &dyn renderpass::IntoDepthAttachment =
                &(&self.depth_texture.view, wgpu::LoadOp::Clear(1.0));

            let mut render_pass =
                renderpass::render_pass(&mut encoder, color_attachments, depth_attachment);

            render_pass.set_pipeline(&self.pipelines.forward);

            self.world
                .render(
                    &self.state,
                    &mut render_pass,
                    &self.uniform_group,
                    &self.light_group,
                )
                .expect("Error rendering");

            render_pass.set_pipeline(&self.pipelines.light);
            render_pass.draw_light_model(&self.obj_model, &self.uniform_group, &self.light_group);

            render_pass.set_pipeline(&self.pipelines.grid);
            render_pass.draw_grid(&self.grid, &self.uniform_group);
        }

        {
            let color_attachments: &[&dyn renderpass::IntoColorAttachment] =
                &[&(&sc.view, wgpu::LoadOp::Load)];

            let mut render_pass = renderpass::render_pass(&mut encoder, color_attachments, None);
            render_pass.set_viewport(0.0, 0.0, 200.0, 200.0, 0.0, 1.0);

            render_pass.set_pipeline(&self.pipelines.depth);

            render_pass.draw_framebuffer(&self.framebuffer, &self.uniform_group);
        }

        {
            if self.last_cursor != ui.mouse_cursor() {
                self.last_cursor = ui.mouse_cursor();
                self.platform.prepare_render(&ui, &self.window);
            }

            let color_attachments: &[&dyn renderpass::IntoColorAttachment] =
                &[&(&sc.view, wgpu::LoadOp::Load)];

            let mut render_pass = renderpass::render_pass(&mut encoder, color_attachments, None);

            self.imgui_renderer
                .render(
                    ui.render(),
                    &self.state.queue(),
                    &self.state.device(),
                    &mut render_pass,
                )
                .expect("Failed to render UI!");
        }

        self.state.queue().submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    #[allow(dead_code)]
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
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
    )
    .unwrap();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Nodas engine")
        .build(&event_loop)
        .unwrap();
    info!("Window intialized");

    let mut engine = block_on(Engine::new(window)).unwrap();
    let mut last_render_time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

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

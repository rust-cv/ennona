use std::time::Duration;
use std::{borrow::Cow, time::Instant};

use bytemuck::{Pod, Zeroable};
use chrono::Timelike;
use egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::App;
use lazy_static::lazy_static;
use nalgebra::{IsometryMatrix3, Matrix4, Point3, Vector3};
use wgpu::Color;
use wgpu::{util::DeviceExt, CommandEncoder, SwapChainError, SwapChainTexture};
use winit::dpi::PhysicalPosition;
use winit::event::{Event, KeyboardInput, WindowEvent};
use winit::window::Window;

use crate::{Application, Camera, CameraController};

// main.rs
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        lazy_static! {
            static ref ATTRIBUTES: [wgpu::VertexAttribute; 2] =
                wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        }
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &*ATTRIBUTES,
        }
    }
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    point_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_points: u32,
    camera: Camera,
    pub camera_controller: CameraController,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    pub platform: Platform,
    egui_render_pass: RenderPass,
    app: Application,
    start_time: Instant,
    previous_frame_time: Option<f32>,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::NON_FILL_POLYGON_MODE,
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let egui_render_pass = RenderPass::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb, 1);

        let start_time = Instant::now();
        let previous_frame_time = None;
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let camera = Camera {
            view_matrix: IsometryMatrix3::identity(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.,
        };

        let camera_controller = CameraController::new(0.02, 1.0);

        let app = Application::new(
            "".into(),
            camera_controller.speed,
            camera_controller.sensitivity,
            size.height,
            size.width,
        );

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let point_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("point shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/shader.wgsl"))),
            flags: wgpu::ShaderFlags::VALIDATION | wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION,
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let point_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &point_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &point_shader,
                entry_point: "fs_main",
                targets: &[sc_desc.format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                polygon_mode: wgpu::PolygonMode::Point,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Vertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 0.0],
            }]),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            point_pipeline,
            vertex_buffer,
            num_points: 0,
            camera,
            camera_controller,
            uniform_bind_group,
            uniform_buffer,
            uniforms,
            platform,
            app,
            egui_render_pass,
            start_time,
            previous_frame_time,
            size,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.app.window_width = new_size.width;
        self.app.window_height = new_size.height;
    }

    pub fn update(&mut self, dt: Duration) {
        self.update_points(dt, self.app.speed, self.app.sensitivity);
        self.update_gui();
    }

    pub fn render(&mut self, scale_factor: f64) -> Result<(), SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        self.platform.begin_frame();
        self.render_gui(scale_factor, &mut encoder, &frame);
        self.render_points(&mut encoder, &frame);

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    fn update_points(&mut self, dt: Duration, new_speed: f32, new_sensitivity: f32) {
        // TODO: update camera controller
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_controller.set_speed(new_speed);
        self.camera_controller.set_sensitivity(new_sensitivity);
        self.uniforms.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    fn render_points(&mut self, encoder: &mut CommandEncoder, frame: &SwapChainTexture) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        if self.num_points != 0 {
            render_pass.set_pipeline(&self.point_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_points, 0..1);
        }
    }

    fn update_gui(&mut self) {
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());
    }

    fn render_gui(
        &mut self,
        scale_factor: f64,
        encoder: &mut CommandEncoder,
        output_frame: &SwapChainTexture,
    ) {
        // Begin to draw the UI frame.
        let egui_start = Instant::now();

        let mut app_output = epi::backend::AppOutput::default();

        let mut frame = epi::backend::FrameBuilder {
            info: epi::IntegrationInfo {
                web_info: None,
                cpu_usage: self.previous_frame_time,
                seconds_since_midnight: Some(seconds_since_midnight()),
                native_pixels_per_point: Some(scale_factor as _),
                prefer_dark_mode: Some(false),
            },
            tex_allocator: &mut self.egui_render_pass,
            output: &mut app_output,
            repaint_signal: std::sync::Arc::new(RepaintSignal),
        }
        .build();

        // Draw the demo application.
        self.app.update(&self.platform.context(), &mut frame);

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_output, paint_commands) = self.platform.end_frame();
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
        self.previous_frame_time = Some(frame_time);

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: self.sc_desc.width,
            physical_height: self.sc_desc.height,
            scale_factor: scale_factor as f32,
        };
        self.egui_render_pass.update_texture(
            &self.device,
            &self.queue,
            &self.platform.context().texture(),
        );
        self.egui_render_pass
            .update_user_textures(&self.device, &self.queue);
        self.egui_render_pass.update_buffers(
            &self.device,
            &self.queue,
            &paint_jobs,
            &screen_descriptor,
        );

        // Record all render passes.
        self.egui_render_pass.execute(
            encoder,
            &output_frame.view,
            &paint_jobs,
            &screen_descriptor,
            Some(Color::BLACK),
        );
    }

    pub fn import_vertices(&mut self, points: &[Vertex]) {
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(points),
                usage: wgpu::BufferUsage::VERTEX,
            });

        self.num_points = points.len() as u32;
    }

    pub fn set_start_position(&mut self, target: Point3<f32>, distance: f32) {
        let d = distance * 5.0;
        self.camera.zfar = 100. * d;
        self.camera.znear = d / 100.;
        self.camera.view_matrix =
            IsometryMatrix3::face_towards(&(target - (d * Vector3::z())), &target, &Vector3::y());
    }

    pub fn input(&mut self, event: &Event<'_, ()>, window: &Window) -> bool {
        match event {
            // capture mouse-move and btn-release as `DeviceEvent`s so we can see them when the pointer leaves the screen
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        },
                    ..
                } => self.camera_controller.process_keyboard(key, *state, window),
                WindowEvent::MouseWheel { delta, .. } => {
                    self.camera_controller.process_scroll(delta);
                    true
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let old_pos = match self.camera_controller.mouse_position.replace(*position) {
                        Some(pos) => pos,
                        None => return false,
                    };
                    let delta_x = position.x - old_pos.x;
                    let delta_y = position.y - old_pos.y;
                    if self.camera_controller.mouse_captured {
                        let size = window.inner_size();
                        let center = PhysicalPosition {
                            x: size.width / 2,
                            y: size.height / 2,
                        };

                        if window.set_cursor_position(center).is_ok() {
                            self.camera_controller
                                .mouse_position
                                .replace(PhysicalPosition {
                                    x: center.x as f64,
                                    y: center.y as f64,
                                });
                        }
                        self.camera_controller.process_mouse(delta_x, delta_y);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }
}

/// Time of day as seconds since midnight.
pub fn seconds_since_midnight() -> f64 {
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
struct RepaintSignal;

impl epi::RepaintSignal for RepaintSignal {
    fn request_repaint(&self) {
        // self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

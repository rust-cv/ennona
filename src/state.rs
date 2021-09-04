use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use chrono::Timelike;
use egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::{App, TextureAllocator};
use image::{DynamicImage, GenericImageView};
use lazy_static::lazy_static;
use nalgebra::{IsometryMatrix3, Matrix4};
use wgpu::{util::DeviceExt, Color, CommandEncoder, SurfaceError, TextureView};
use winit::window::Window;

use crate::{face::FaceState, import::PlyData, Camera, Interface};

// main.rs
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        lazy_static! {
            static ref ATTRIBUTES: [wgpu::VertexAttribute; 2] =
                wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        }
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
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
    surface_config: wgpu::SurfaceConfiguration,
    point_pipeline: wgpu::RenderPipeline,
    point_vertex_buffer: wgpu::Buffer,
    num_points: u32,
    face_state: FaceState,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    pub platform: Platform,
    egui_render_pass: RenderPass,
    start_time: Instant,
    previous_frame_time: Option<f32>,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

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

        let point_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("point shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/points.wgsl").into()),
        });

        let point_shader_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Point Shader Uniform Bind Group Layout"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&point_shader_uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

        let point_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Point Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &point_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &point_shader,
                entry_point: "fs_main",
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                polygon_mode: wgpu::PolygonMode::Point,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        surface.configure(&device, &surface_config);

        let start_time = Instant::now();
        let previous_frame_time = None;

        let uniforms = Uniforms::new();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("View-Projection Matrix"),
            contents: bytemuck::cast_slice(&[Uniforms::new()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &point_shader_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&point_shader_uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let point_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Vertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let face_state = FaceState::new(
            swapchain_format.into(),
            &device,
            &render_pipeline_layout,
            &point_shader,
        );

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        let egui_render_pass = RenderPass::new(&device, swapchain_format, 1);

        Self {
            surface,
            device,
            queue,
            surface_config,
            point_pipeline,
            point_vertex_buffer,
            face_state,
            num_points: 0,
            uniform_bind_group,
            uniform_buffer,
            uniforms,
            platform,
            egui_render_pass,
            start_time,
            previous_frame_time,
            size,
        }
    }

    /// Updates the size of the display and rebuilds the swapchain.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn update(&mut self, camera: &Camera) {
        self.update_view_proj(camera);
        self.update_gui();
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.uniforms.view_proj = camera.build_view_projection_matrix().into();
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    pub fn render(&mut self, app: &mut Interface, scale_factor: f64) -> Result<(), SurfaceError> {
        let frame = self.surface.get_current_frame()?.output;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        self.platform.begin_frame();
        self.render_gui(app, scale_factor, &mut encoder, &view);
        self.render_points(&mut encoder, &view);

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    pub fn create_initial_camera(&self) -> Camera {
        Camera {
            view_matrix: IsometryMatrix3::identity(),
            aspect: self.surface_config.width as f32 / self.surface_config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.,
        }
    }

    fn render_points(&mut self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view,
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
            render_pass.set_vertex_buffer(0, self.point_vertex_buffer.slice(..));
            render_pass.draw(0..self.num_points, 0..1);
        }
        if self.face_state.num_indices != 0 {
            render_pass.set_pipeline(&self.face_state.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.face_state.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.face_state.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..self.face_state.num_indices, 0, 0..1);
        }
    }

    fn update_gui(&mut self) {
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());
    }

    fn render_gui(
        &mut self,
        app: &mut Interface,
        scale_factor: f64,
        encoder: &mut CommandEncoder,
        view: &TextureView,
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
        app.update(&self.platform.context(), &mut frame);

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_output, paint_commands) = self.platform.end_frame(None);
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
        self.previous_frame_time = Some(frame_time);

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: self.surface_config.width,
            physical_height: self.surface_config.height,
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
        self.egui_render_pass
            .execute(
                encoder,
                view,
                &paint_jobs,
                &screen_descriptor,
                Some(Color::BLACK),
            )
            .expect("egui backend error");
    }

    pub fn import_ply_data(&mut self, ply: &PlyData) {
        let contents = bytemuck::cast_slice(&ply.point_vertices);
        self.point_vertex_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Point Vertex Buffer"),
                    contents,
                    usage: wgpu::BufferUsages::VERTEX,
                });

        self.num_points = ply.point_vertices.len() as u32;
        self.face_state.import_faces(&self.device, ply);
    }

    pub fn import_image(&mut self, img: DynamicImage, interface: &mut Interface) {
        let width = img.width();
        let height = img.height();
        let buff = img.to_rgba8().to_vec();
        // Note: egui_wgpu will convert the image back to Vec<u8> in RenderPass::alloc_srgba_premultiplied so this step can  be skipped
        let srgba_pixels: Vec<egui::Color32> = buff
            .chunks(4)
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();
        let texture_id = self
            .egui_render_pass
            .alloc_srgba_premultiplied((width as usize, height as usize), &srgba_pixels[..]);
        interface.add_image(texture_id, (width as f32, height as f32));
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
}

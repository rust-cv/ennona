mod face_renderer;
mod gui_renderer;
mod point_renderer;

use crate::{import::PlyData, Camera, Interface};
use bytemuck::{Pod, Zeroable};
use egui::TextureId;
use face_renderer::FaceRenderer;
use gui_renderer::GuiRenderer;
use image::DynamicImage;
use nalgebra::Matrix4;
use point_renderer::PointRenderer;
use wgpu::{util::DeviceExt, Color, SurfaceConfiguration, SurfaceError};
use winit::{event::Event, window::Window};

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: SurfaceConfiguration,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    point_renderer: PointRenderer,
    face_renderer: FaceRenderer,
    gui_renderer: GuiRenderer,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        // Creates a new WGPU instance by initalizing with a given backend (e.g. Vulkan) at runtime.
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

        // Get the window surface and its size.
        let surface = unsafe { instance.create_surface(window) };
        let size = window.inner_size();

        // Retreives a specific GPU. We ask for a GPU with high performance for this task.
        // The GPU must also be compatible with the window surface.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // Get the device and queue.
        // These are used to interface with the chosen GPU.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::POLYGON_MODE_POINT
                        | wgpu::Features::POLYGON_MODE_LINE,
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        // Configure the window surface texture.
        let target_texture_format = surface.get_preferred_format(&adapter).unwrap();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: target_texture_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &surface_config);

        // Create the uniforms.
        let uniforms = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("View-Projection Matrix"),
            contents: bytemuck::cast_slice(&[Uniforms::new()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create all of the renderers.
        let point_renderer = PointRenderer::new(&device, &uniform_buffer, target_texture_format);
        let face_renderer = FaceRenderer::new(&device, &uniform_buffer, target_texture_format);
        let gui_renderer = GuiRenderer::new(window, &device, target_texture_format, size);

        Self {
            surface,
            device,
            queue,
            surface_config,
            uniforms,
            uniform_buffer,
            point_renderer,
            face_renderer,
            gui_renderer,
        }
    }

    /// Updates the size of the display and rebuilds the swapchain.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Renders the entire frame.
    pub fn render(
        &mut self,
        app: &mut Interface,
        camera: &Camera,
        scale_factor: f64,
    ) -> Result<(), SurfaceError> {
        // Updates the uniforms from the camera.
        self.uniforms.view_proj = camera.build_view_projection_matrix().into();
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
        let frame = self.surface.get_current_texture()?;

        // Get the texture of the window so we can render on it.
        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create a command encoder which will buffer GPU commands.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Clears the screen to black when this render pass executes.
        // Create a render pass with NO depth attachment.
        // TODO: Attach a depth buffer.
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        // Render the points.
        self.point_renderer.render(&mut render_pass);

        // Render the faces.
        self.face_renderer.render(&mut render_pass);

        drop(render_pass);

        // Clear the screen to black and render the GUI.
        self.gui_renderer.render(
            &self.device,
            &self.queue,
            &mut encoder,
            &target,
            app,
            scale_factor,
            &self.surface_config,
        );

        // Submit the command buffer to the queue to run everything.
        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    /// Imports data from a PLY file.
    pub fn import_ply(&mut self, ply: &PlyData) {
        self.point_renderer.import_ply(&self.device, ply);
        self.face_renderer.import_ply(&self.device, ply);
    }

    /// Creates a texture to use with `egui`.
    pub fn make_egui_texture(&mut self, image: DynamicImage) -> TextureId {
        self.gui_renderer.make_texture(image)
    }

    /// Returns if the event was caputured by the gui.
    pub fn handle_event(&mut self, event: &Event<'_, ()>) -> bool {
        self.gui_renderer.handle_event(event)
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

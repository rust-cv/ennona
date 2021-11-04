use std::time::Instant;

use egui::{FontDefinitions, TextureId};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::{App, TextureAllocator};
use image::{DynamicImage, GenericImageView};
use wgpu::{CommandEncoder, Device, Queue, SurfaceConfiguration, TextureFormat, TextureView};
use winit::{dpi::PhysicalSize, event::Event, window::Window};

use crate::interface::Interface;

pub struct GuiRenderer {
    platform: Platform,
    egui_render_pass: RenderPass,
    previous_frame_time: Option<f32>,
    start_time: Instant,
}

impl GuiRenderer {
    pub fn new(
        window: &Window,
        device: &Device,
        target_texture_format: TextureFormat,
        size: PhysicalSize<u32>,
    ) -> Self {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        let egui_render_pass = RenderPass::new(device, target_texture_format, 1);

        // Store everything in the renderer.
        Self {
            platform,
            egui_render_pass,
            previous_frame_time: None,
            start_time: Instant::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        target: &TextureView,
        app: &mut Interface,
        scale_factor: f64,
        surface_config: &SurfaceConfiguration,
    ) {
        self.platform
            .update_time(self.start_time.elapsed().as_secs_f64());
        self.platform.begin_frame();

        // Begin to draw the UI frame.
        let egui_start = Instant::now();

        let mut app_output = epi::backend::AppOutput::default();
        let mut frame = epi::backend::FrameBuilder {
            info: epi::IntegrationInfo {
                name: "Ennona",
                web_info: None,
                cpu_usage: self.previous_frame_time,
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
            physical_width: surface_config.width,
            physical_height: surface_config.height,
            scale_factor: scale_factor as f32,
        };
        self.egui_render_pass
            .update_texture(device, queue, &self.platform.context().texture());
        self.egui_render_pass.update_user_textures(device, queue);
        self.egui_render_pass
            .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

        // Record all render passes.
        self.egui_render_pass
            .execute(encoder, target, &paint_jobs, &screen_descriptor, None)
            .expect("egui backend error");
    }

    pub fn make_texture(&mut self, image: DynamicImage) -> TextureId {
        // Note: egui_wgpu will convert the image back to Vec<u8> in RenderPass::alloc_srgba_premultiplied so this step can be skipped
        let width = image.width();
        let height = image.height();
        let buff = image.to_rgba8().to_vec();
        let srgba_pixels: Vec<egui::Color32> = buff
            .chunks(4)
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();
        self.egui_render_pass
            .alloc_srgba_premultiplied((width as usize, height as usize), &srgba_pixels[..])
    }

    /// Returns if the event was caputured by the gui.
    pub fn handle_event(&mut self, event: &Event<'_, ()>) -> bool {
        self.platform.handle_event(event);
        self.platform.captures_event(event)
    }
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
struct RepaintSignal;

impl epi::RepaintSignal for RepaintSignal {
    fn request_repaint(&self) {
        // self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}

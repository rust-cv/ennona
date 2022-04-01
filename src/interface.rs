use std::time::Duration;

use egui::{Button, CollapsingHeader, Frame, Slider, Stroke, TextureId, Ui};
use winit::{
    event::{KeyboardInput, WindowEvent},
    window::Window,
};

use crate::{camera::Camera, CameraController};

pub struct ImageTextureId {
    pub texture_id: TextureId,
    pub size: egui::Vec2,
}

pub struct Interface {
    // Example stuff:
    pub file_name: String,
    pub window_width: u32,
    pub window_height: u32,
    pub camera_controller: CameraController,
    pub images: Vec<ImageTextureId>,
    pub displayed_image_idx: u32,
    pub camera_scale: f32,
}

impl Interface {
    pub fn new(file_name: String, window_width: u32, window_height: u32) -> Self {
        Self {
            file_name,
            window_width,
            window_height,
            camera_controller: CameraController::new(0.5, 0.000818123),
            images: Vec::new(),
            displayed_image_idx: 0,
            camera_scale: 1.0,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_width = new_size.width;
        self.window_height = new_size.height;
    }

    pub fn update_camera(&self, camera: &mut Camera, dt: Duration) {
        self.camera_controller.update_camera(camera, dt);
    }

    pub fn set_camera_scale(&mut self, scale: f32) {
        self.camera_scale = scale;
        self.camera_controller.speed = scale * 5.0;
    }

    pub fn add_image(&mut self, texture_id: TextureId, size: (f32, f32)) {
        self.images.push(ImageTextureId {
            texture_id,
            size: size.into(),
        });
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        Frame::popup(ui.style())
            .stroke(Stroke::none())
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() * 0.20);
                CollapsingHeader::new("Settings").show(ui, |ui| self.options_ui(ui))
            });
    }

    fn options_ui(&mut self, ui: &mut Ui) {
        if self.file_name.is_empty() {}
        if ui.add(Button::new("File")).clicked() {
            // do nothing right now
        }
        ui.add(
            Slider::new(
                &mut self.camera_controller.speed,
                self.camera_scale..=25.0 * self.camera_scale,
            )
            .text("speed")
            .clamp_to_range(true)
            .logarithmic(true),
        );
        ui.add(
            Slider::new(&mut self.camera_controller.sensitivity, 0.0001..=0.003)
                .text("sensitivity")
                .clamp_to_range(true)
                .logarithmic(true),
        );
        ui.label(format!("Window width: {}", self.window_width));
        ui.label(format!("Window height: {}", self.window_height));

        if self.images.len() > 1 {
            ui.add(
                Slider::new(
                    &mut self.displayed_image_idx,
                    0..=self.images.len() as u32 - 1,
                )
                .text("image")
                .clamp_to_range(true)
                .prefix("#"),
            );
        }
        if let Some(img) = self.images.get(self.displayed_image_idx as usize) {
            let width = 360f32;
            let height = img.size.y * (width / img.size.x);
            ui.image(img.texture_id, (width, height));
        }
    }

    pub fn input(&mut self, event: &WindowEvent<'_>, window: &Window) {
        // capture mouse-move and btn-release as `DeviceEvent`s so we can see them when the pointer leaves the screen
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(key, *state, window),
            WindowEvent::MouseWheel { delta, .. } => self.camera_controller.process_scroll(delta),
            _ => {}
        }
    }
}

impl epi::App for Interface {
    fn name(&self) -> &str {
        "⛅ Ennona"
    }

    /// Called by the framework to load old app state (if any).
    // #[cfg(feature = "persistence")]
    // fn setup(
    //     &mut self,
    //     _ctx: &egui::CtxRef,
    //     _frame: &mut epi::Frame<'_>,
    //     storage: Option<&dyn epi::Storage>,
    // ) {
    //     *self = epi::get_value(storage.unwrap(), epi::APP_KEY).unwrap_or_default()
    // }

    /// Called by the frame work to save state before shutdown.
    // #[cfg(feature = "persistence")]
    // fn save(&mut self, storage: &mut dyn epi::Storage) {
    //     epi::set_value(storage, epi::APP_KEY, self);
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::Area::new("settings").show(ctx, |ui| self.ui(ui));
    }
}

use std::time::Duration;

use egui::{Button, CollapsingHeader, Frame, Slider, Stroke, Ui};
use winit::{
    dpi::PhysicalPosition,
    event::{Event, KeyboardInput, WindowEvent},
    window::Window,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
// #[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
use crate::{camera::Camera, CameraController};

pub struct Interface {
    // Example stuff:
    pub file_name: String,
    pub window_width: u32,
    pub window_height: u32,
    pub camera_controller: CameraController,
}

impl Interface {
    pub fn new(file_name: String, window_width: u32, window_height: u32) -> Self {
        Self {
            file_name,
            window_width,
            window_height,
            camera_controller: CameraController::new(0.02, 1.0),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_width = new_size.width;
        self.window_height = new_size.height;
    }

    pub fn update_camera(&self, camera: &mut Camera, dt: Duration) {
        self.camera_controller.update_camera(camera, dt);
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
            Slider::new(&mut self.camera_controller.speed, 0.0005..=0.05)
                .text("speed")
                .clamp_to_range(true)
                .logarithmic(true),
        );
        ui.add(
            Slider::new(&mut self.camera_controller.sensitivity, 0.0..=5.0)
                .text("sensitivity")
                .clamp_to_range(true)
                .logarithmic(true),
        );
        ui.label(format!("Window width: {}", self.window_width));
        ui.label(format!("Window height: {}", self.window_height));
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
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::Area::new("settings").show(ctx, |ui| self.ui(ui));
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
// #[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
use egui::{containers::*, widgets::*, *};
pub struct Application {
    // Example stuff:
    file_name: String,
    zoom: f32,
    window_height: u32,
    window_width: u32,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            // Example stuff:
            file_name: "".to_owned(),
            zoom: 0.0,
            window_height: 600,
            window_width: 800,
        }
    }
}

impl Application {
    pub fn new(file_name: String, zoom: f32, window_height: u32, window_width: u32) -> Self {
        Self {
            file_name,
            zoom,
            window_height,
            window_width,
        }
    }
}

impl epi::App for Application {
    fn name(&self) -> &str {
        "⛅ Cloud"
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
        egui::CentralPanel::default()
            .frame(Frame::dark_canvas(&ctx.style()))
            .show(ctx, |ui| self.ui(ui));
    }
}

impl Application {
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
        ui.add(Slider::new(&mut self.zoom, -100.0..=100.0).text("zoom"));
        ui.label(format!("Window width: {}", self.window_width));
        ui.label(format!("Window height: {}", self.window_height));
    }
}

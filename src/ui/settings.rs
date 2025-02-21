use super::background_rect::start_ocr_id;
use crate::action::open_workdir;
use crate::ui::event::Event::ResetUi;
use crate::ui::event::EventHandler;
use egui::{Button, CollapsingHeader, Color32, Id, RichText, Spinner};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub clear_color: Color32,
    pub mouse_passthrough: bool,
    pub decorations: bool,
    pub zoom_factor: f32,

    pub auto_restart_ocr: bool,
    pub auto_restart_delay_ms: u64,
    pub hover_delay_ms: u64,

    //OCR Settings
    pub detect_boxes: bool,

    pub show_statistics: bool,
    pub show_history: bool,
    pub show_capture_image: bool,
    pub show_debug_image: bool,
    pub threshold: f32,

    pub show_debug_cursor: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clear_color: Color32::TRANSPARENT,
            mouse_passthrough: false,
            decorations: false,
            detect_boxes: true,
            zoom_factor: 1.5,
            auto_restart_ocr: true,
            auto_restart_delay_ms: 1000,
            hover_delay_ms: 1000,
            show_statistics: false,
            show_history: false,
            show_capture_image: false,
            show_debug_image: false,
            threshold: 0.5,
            show_debug_cursor: false,
        }
    }
}

impl AppSettings {
    pub(crate) fn show(&mut self, ctx: &egui::Context) {
        let window = egui::Window::new("Settings")
            .default_width(50.0)
            .resizable(false);
        window.show(ctx, |ui| {
            self.show_window_settings(ui);

            ui.horizontal(|ui| {
                Backend::MangaOcr.get_status_ui(ui);
                let enabled = Backend::MangaOcr.get_status(ui) == BackendStatus::Ready;
                if ui.add_enabled(enabled, Button::new("Start OCR")).clicked() {
                    ui.data_mut(|map| map.insert_temp(start_ocr_id(), true));
                }
                ui.checkbox(&mut self.auto_restart_ocr, "Auto Restart OCR");
            });

            self.show_ocr_config(ui);
            self.show_debug_config(ui);

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(format!("{:^15}", "Quit")).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ui.add_space(80.0);
                ui.hyperlink_to(
                    "\u{E624} Manga Overlay on GitHub",
                    "https://github.com/Icekey/manga-overlay",
                );
            });
        });
    }

    fn show_window_settings(&mut self, ui: &mut egui::Ui) {
        egui::widgets::global_theme_preference_buttons(ui);

        ui.horizontal(|ui| {
            ui.label("Zoom Factor:");
            ui.selectable_value(&mut self.zoom_factor, 1.0, "100%");
            ui.selectable_value(&mut self.zoom_factor, 1.5, "150%");
            ui.selectable_value(&mut self.zoom_factor, 2.0, "200%");
            ui.selectable_value(&mut self.zoom_factor, 2.5, "250%");
            ui.selectable_value(&mut self.zoom_factor, 3.0, "300%");
        });

        ui.checkbox(&mut self.mouse_passthrough, "Mouse Passthrough");

        if ui.checkbox(&mut self.decorations, "Decorations").clicked() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Decorations(self.decorations));
        }

        ui.checkbox(&mut self.show_history, "Show History");
        ui.checkbox(&mut self.show_statistics, "Show Statistics");
    }

    fn show_ocr_config(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("OCR Config").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.detect_boxes, false, "Full Capture");
                ui.selectable_value(&mut self.detect_boxes, true, "Detect Boxes");
            });
            ui.horizontal(|ui| {
                if !self.detect_boxes {
                    ui.disable()
                }
                ui.add(egui::Slider::new(&mut self.threshold, 0.0..=1.0).text("Box Threshold"));
            });

            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.auto_restart_delay_ms, 0..=5000)
                        .text("Auto Restart Time (ms)"),
                );
            });

            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.hover_delay_ms, 0..=5000).text("Hover Delay (ms)"),
                );
            });
        });
    }

    fn show_debug_config(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Debug Config").show(ui, |ui| {
            if ui.button("Open Workdir").clicked() {
                open_workdir();
            }

            ui.horizontal(|ui| {
                ui.label("Background Color: ");
                ui.color_edit_button_srgba(&mut self.clear_color);
            });

            ui.checkbox(&mut self.show_capture_image, "Show Capture Image");
            ui.checkbox(&mut self.show_debug_image, "Show Debug Image");
            ui.checkbox(&mut self.show_debug_cursor, "Show Debug Cursor");

            if ui.button("Reset UI").clicked() {
                ui.ctx().emit(ResetUi);
            }
        });
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BackendStatus {
    Loading,
    Ready,
    Running,
    Error,
}

impl BackendStatus {
    fn get_ui(&self, ui: &mut egui::Ui) {
        match self {
            BackendStatus::Loading => ui.add(Spinner::new()),
            BackendStatus::Ready | BackendStatus::Running => {
                ui.label(RichText::from("\u{2714}").color(Color32::GREEN))
            }
            BackendStatus::Error => ui.label(RichText::from("\u{2716}").color(Color32::RED)),
        };
    }
}

#[derive(Debug, Clone)]
pub enum Backend {
    MangaOcr,
}

impl Backend {
    fn get_id(self: &Backend) -> Id {
        match self {
            Backend::MangaOcr => Id::new("MangaOcr_Status"),
        }
    }

    fn get_status(&self, ui: &egui::Ui) -> BackendStatus {
        ui.data(|data| {
            data.get_temp(self.get_id())
                .unwrap_or_else(|| BackendStatus::Loading)
        })
    }

    fn get_status_ui(&self, ui: &mut egui::Ui) {
        self.get_status(ui).get_ui(ui);
    }

    pub fn set_status(&self, ctx: &egui::Context, status: BackendStatus) {
        ctx.data_mut(|data| data.insert_temp(self.get_id(), status));
    }
}

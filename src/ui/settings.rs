use super::background_rect::start_ocr_id;
use crate::{action::open_workdir, ocr::TesseractParameter};
use egui::{CollapsingHeader, Color32, Id, RichText, Spinner};
use log::info;
use rusty_tesseract::get_tesseract_langs;

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

    pub is_tesseract: bool,
    pub tesseract_parameter: TesseractParameter,

    pub is_manga_ocr: bool,

    #[serde(skip)]
    pub langs: Vec<String>,

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
            is_tesseract: false,
            tesseract_parameter: TesseractParameter::default(),
            is_manga_ocr: true,
            langs: get_tesseract_langs().unwrap_or_default(),
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
        let window = egui::Window::new("Settings").resizable(false);
        window.show(ctx, |ui| {
            self.show_window_settings(ui);

            self.show_ocr_config(ui);
            self.show_debug_config(ui);

            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
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

                if !self.detect_boxes {
                    ui.disable()
                }
                ui.add(egui::Slider::new(&mut self.threshold, 0.0..=1.0).text("Box Threshold"));
            });

            ui.horizontal(|ui| {
                if ui.button("Start OCR").clicked() {
                    info!("Start OCR");
                    ui.data_mut(|map| map.insert_temp(start_ocr_id(), true));
                }
                ui.checkbox(&mut self.auto_restart_ocr, "Auto Restart OCR");
            });
            ui.horizontal(|ui| {
                ui.label("Auto Restart Time(ms):");

                ui.add(egui::Slider::new(&mut self.auto_restart_delay_ms, 0..=5000));
            });

            ui.horizontal(|ui| {
                ui.label("Hover Delay(ms):");

                ui.add(egui::Slider::new(&mut self.hover_delay_ms, 0..=5000));
            });

            ui.separator();

            //MangaOcr
            self.show_manga_ocr_config(ui);
            ui.separator();

            //Tesseract
            self.show_tesseract_config(ui);
        });
    }

    fn show_tesseract_config(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.is_tesseract, "Tesseract");
            Backend::Tesseract.get_ui(ui);
        });
        if self.is_tesseract {
            ui.horizontal(|ui| {
                ui.label("DPI:");

                ui.add(egui::Slider::new(
                    &mut self.tesseract_parameter.dpi,
                    10..=500,
                ));
            });

            ui.horizontal(|ui| {
                ui.label("Language:");
                egui::ComboBox::from_id_salt(Id::new("tesseract_lang"))
                    .selected_text(self.tesseract_parameter.lang.clone())
                    .show_ui(ui, |ui| {
                        for lang in &self.langs {
                            ui.selectable_value(
                                &mut self.tesseract_parameter.lang,
                                lang.to_string(),
                                lang,
                            );
                        }
                    });
            });
        }
    }

    fn show_manga_ocr_config(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.is_manga_ocr, "MangaOcr");
            Backend::MangaOcr.get_ui(ui);
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
        });
    }
}

#[derive(Debug, Clone)]
pub enum BackendStatus {
    Loading,
    Ready,
    Error,
}

impl BackendStatus {
    fn get_ui(&self, ui: &mut egui::Ui) {
        match self {
            BackendStatus::Loading => ui.add(Spinner::new()),
            BackendStatus::Ready => ui.label(RichText::from("\u{2714}").color(Color32::GREEN)),
            BackendStatus::Error => ui.label(RichText::from("\u{2716}").color(Color32::RED)),
        };
    }
}

#[derive(Debug, Clone)]
pub enum Backend {
    Tesseract,
    MangaOcr,
}

impl Backend {
    fn get_id(self: &Backend) -> Id {
        match self {
            Backend::Tesseract => Id::new("Tesseract_Status"),
            Backend::MangaOcr => Id::new("MangaOcr_Status"),
        }
    }

    fn get_status(&self, ui: &egui::Ui) -> BackendStatus {
        ui.data(|data| {
            data.get_temp(self.get_id())
                .unwrap_or_else(|| BackendStatus::Loading)
        })
    }

    fn get_ui(&self, ui: &mut egui::Ui) {
        self.get_status(ui).get_ui(ui);
    }

    pub fn set_status(&self, ctx: &egui::Context, status: BackendStatus) {
        ctx.data_mut(|data| data.insert_temp(self.get_id(), status));
    }
}

use egui::{CollapsingHeader, Id};
use rusty_tesseract::get_tesseract_langs;

use crate::ocr::{EasyOcrParameter, TesseractParameter};

use super::show_ui::ShowUi;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppSettings {
    pub clear_color: egui::Color32,
    pub mouse_passthrough: bool,
    pub decorations: bool,

    //OCR Settings
    pub detect_boxes: bool,

    pub is_tesseract: bool,
    pub tesseract_parameter: TesseractParameter,

    pub is_easy_ocr: bool,
    pub easy_ocr_parameter: EasyOcrParameter,

    pub is_manga_ocr: bool,

    pub langs: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clear_color: Default::default(),
            mouse_passthrough: Default::default(),
            decorations: Default::default(),
            is_tesseract: Default::default(),
            tesseract_parameter: Default::default(),
            is_easy_ocr: Default::default(),
            easy_ocr_parameter: Default::default(),
            is_manga_ocr: Default::default(),
            langs: get_tesseract_langs().unwrap_or_default(),
            detect_boxes: true,
        }
    }
}

impl ShowUi for AppSettings {
    fn show(&mut self, ctx: &egui::Context) {
        let window = egui::Window::new("Settings");
        window.show(ctx, |ui| {
            egui::widgets::global_theme_preference_buttons(ui);

            ui.horizontal(|ui| {
                ui.label("Background Color: ");
                ui.color_edit_button_srgba(&mut self.clear_color);
            });

            ui.checkbox(&mut self.mouse_passthrough, "Mouse Passthrough");

            if ui.checkbox(&mut self.decorations, "Decorations").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(self.decorations));
            }

            self.show_ocr_config(ui);

            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

impl AppSettings {
    fn show_ocr_config(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("OCR Config").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.detect_boxes, false, "Full Capture");
                ui.selectable_value(&mut self.detect_boxes, true, "Detect Boxes");
            });

            ui.separator();

            //Tesseract
            ui.checkbox(&mut self.is_tesseract, "Tesseract");
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
            ui.separator();

            //EasyOcr
            ui.checkbox(&mut self.is_easy_ocr, "EasyOcr");
            if self.is_easy_ocr {
                ui.horizontal(|ui| {
                    ui.label("Language:");
                    egui::ComboBox::from_id_salt(Id::new("easy_ocr_lang"))
                        .selected_text(self.easy_ocr_parameter.lang.clone())
                        .show_ui(ui, |ui| {
                            for lang in &self.langs {
                                ui.selectable_value(
                                    &mut self.easy_ocr_parameter.lang,
                                    lang.to_string(),
                                    lang,
                                );
                            }
                        });
                });
            }

            ui.separator();
            //MangaOcr
            ui.checkbox(&mut self.is_manga_ocr, "MangaOcr");
        });
    }
}

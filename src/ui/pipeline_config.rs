use crate::action::OcrPipelineStep;
use crate::ocr::OcrBackend;
use crate::ui::id_item::{IdItem, IdItemVec};
use crate::ui::settings::PreprocessConfig;
use egui::{CollapsingHeader, Ui};
use egui_dnd::{DragDropItem, dnd};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct OcrPipeline {
    pub items: Vec<IdItem<OcrPipelineStep>>,
    active: bool,
    new_step_selected: OcrPipelineStep,
    new_step_combobox: Vec<OcrPipelineStep>,
}

impl Default for OcrPipeline {
    fn default() -> Self {
        let vec = vec![
            OcrPipelineStep::ImageProcessing(PreprocessConfig::default()),
            OcrPipelineStep::BoxDetection { threshold: 0.5 },
            OcrPipelineStep::OcrStep {
                backend: OcrBackend::MangaOcr,
            },
        ];
        OcrPipeline {
            items: IdItem::from_vec(vec.clone()),
            active: false,
            new_step_selected: OcrPipelineStep::ImageProcessing(PreprocessConfig::default()),
            new_step_combobox: vec,
        }
    }
}

impl OcrPipeline {
    pub fn show(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("OCR Pipeline Config").show(ui, |ui| {
            dnd(ui, "dnd_pipeline").show_vec(&mut self.items, |ui, item, handle, _state| {
                ui.horizontal(|ui| {
                    handle.ui(ui, |ui| {
                        ui.label("\u{2B0D}");
                    });

                    ui.checkbox(&mut item.active, "");
                    item.show(ui);
                });
                ui.separator();
            });
            ui.horizontal(|ui| {
                if ui.button("\u{229E}").clicked() {
                    self.items.push_item(self.new_step_selected.clone());
                };

                egui::ComboBox::from_label("Add Step")
                    .selected_text(self.new_step_selected.header_name().to_string())
                    .show_ui(ui, |ui| {
                        for step in &mut self.new_step_combobox {
                            ui.selectable_value(
                                &mut self.new_step_selected,
                                step.clone(),
                                step.header_name(),
                            );
                        }
                    });
            });
        });
    }
}

impl IdItem<OcrPipelineStep> {
    pub fn show(&mut self, ui: &mut Ui) {
        CollapsingHeader::new(self.item.header_name())
            .id_salt(&self.id())
            .show(ui, |ui| {
                ui.add_enabled_ui(self.active, |ui| self.item.show(ui))
            });
    }
}

impl OcrPipelineStep {
    fn show(&mut self, ui: &mut Ui) {
        match self {
            OcrPipelineStep::ImageProcessing(config) => config.show(ui),
            OcrPipelineStep::BoxDetection { threshold } => Self::show_box_detection(ui, threshold),
            OcrPipelineStep::OcrStep { backend } => Self::show_ocr_step(ui, backend),
        }
    }

    fn show_box_detection(ui: &mut Ui, threshold: &mut f32) {
        ui.add(egui::Slider::new(threshold, 0.0..=1.0).text("Box Threshold"));
    }

    fn show_ocr_step(ui: &mut Ui, backend: &mut OcrBackend) {
        ui.label(backend.to_string());
    }

    fn header_name(&self) -> &'static str {
        match self {
            OcrPipelineStep::ImageProcessing(_) => "Image Processing",
            OcrPipelineStep::BoxDetection { .. } => "Box Detection",
            OcrPipelineStep::OcrStep { .. } => "OCR Step",
        }
    }
}

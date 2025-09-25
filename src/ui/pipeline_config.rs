use crate::action::OcrPipelineStep;
use crate::event::event::Event::RemovePipelineStep;
use crate::event::event::emit_event;
use crate::ui::id_item::IdItem;
use eframe::epaint::Color32;
use egui::{CollapsingHeader, RichText, Ui};
use egui_dnd::dnd;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct OcrPipeline {
    pub items: Vec<IdItem<OcrPipelineStep>>,
    pub name: String,
}

impl Default for OcrPipeline {
    fn default() -> Self {
        Self {
            items: IdItem::from_vec(vec![OcrPipelineStep::BoxDetection {
                threshold: 0.5,
                max_box_count: 10,
                use_capture_image_as_output: true,
            }]),
            name: "Box Detection".to_string(),
        }
    }
}

impl OcrPipeline {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(egui::TextEdit::singleline(&mut self.name));
        });
        dnd(ui, "dnd_pipeline").show_vec(&mut self.items, |ui, item, handle, state| {
            ui.horizontal(|ui| {
                handle.ui(ui, |ui| {
                    ui.label(format!("{} \u{2B0D}", state.index + 1));
                });

                ui.checkbox(&mut item.active, "");

                item.show(ui, state.index);

                if ui
                    .button(RichText::new("\u{1F5D9}").color(Color32::RED))
                    .clicked()
                {
                    emit_event(RemovePipelineStep(state.index))
                }
            });

            ui.separator();
        });
    }
}

impl IdItem<OcrPipelineStep> {
    pub fn show(&mut self, ui: &mut Ui, index: usize) {
        if self.item.has_parameters() {
            CollapsingHeader::new((&self.item).name())
                .id_salt(index)
                .show(ui, |ui| self.item.show(ui));
        } else {
            ui.label((&self.item).name());
        }
    }
}

impl OcrPipelineStep {
    fn show(&mut self, ui: &mut Ui) {
        match self {
            OcrPipelineStep::ImageProcessing(config) => config.show(ui),
            OcrPipelineStep::BoxDetection {
                threshold,
                max_box_count,
                use_capture_image_as_output,
            } => {
                Self::show_box_detection(ui, threshold, max_box_count, use_capture_image_as_output)
            }
            _ => {}
        }
    }

    fn has_parameters(&self) -> bool {
        match self {
            OcrPipelineStep::ImageProcessing(_) | OcrPipelineStep::BoxDetection { .. } => true,
            _ => false,
        }
    }

    fn show_box_detection(
        ui: &mut Ui,
        threshold: &mut f32,
        max_box_count: &mut usize,
        use_capture_image_as_output: &mut bool,
    ) {
        ui.add(egui::Slider::new(threshold, 0.0..=1.0).text("Box Threshold"));
        ui.add(egui::Slider::new(max_box_count, 1..=100).text("Max Box Count"));
        ui.checkbox(
            &mut *use_capture_image_as_output,
            "Use Capture Image Output",
        );
    }
}

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
    pub active: bool,
    pub name: String,
}

impl OcrPipeline {
    pub fn show(&mut self, ui: &mut Ui) {
        dnd(ui, "dnd_pipeline").show_vec(&mut self.items, |ui, item, handle, state| {
            ui.horizontal(|ui| {
                handle.ui(ui, |ui| {
                    ui.label(format!("{} \u{2B0D}", state.index + 1));
                });

                ui.checkbox(&mut item.active, "");

                item.show(ui);

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
    pub fn show(&mut self, ui: &mut Ui) {
        if self.item.has_parameters() {
            CollapsingHeader::new((&self.item).name()).show(ui, |ui| self.item.show(ui));
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
                use_capture_image_as_output,
            } => Self::show_box_detection(ui, threshold, use_capture_image_as_output),
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
        use_capture_image_as_output: &mut bool,
    ) {
        ui.add(egui::Slider::new(threshold, 0.0..=1.0).text("Box Threshold"));
        ui.checkbox(
            &mut *use_capture_image_as_output,
            "Use Capture Image Output",
        );
    }
}

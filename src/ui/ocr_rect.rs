use super::show_ui::ShowUi;
use egui::{Color32, Id, Rect};

pub struct OcrUiRect {
    label: String,
    rect: Rect,
    color: Color32,
}

impl Default for OcrUiRect {
    fn default() -> Self {
        Self {
            label: Default::default(),
            rect: Rect::NAN,
            color: Color32::RED,
        }
    }
}

impl OcrUiRect {
    pub fn new(label: String) -> Self {
        Self {
            label,
            ..Default::default()
        }
    }

    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = rect;
        self
    }
}

impl ShowUi for OcrUiRect {
    fn show(&mut self, ctx: &egui::Context) {
        egui::Area::new(Id::new(self.label.clone()))
            .order(egui::Order::Debug)
            .show(ctx, |ui| {
                ui.painter()
                    .rect(self.rect, 0.0, Color32::TRANSPARENT, (1.0, self.color));
            });
    }
}

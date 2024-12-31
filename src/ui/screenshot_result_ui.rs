use egui::{Color32, Id, Pos2, Rect, RichText, Sense, Vec2};
use log::info;

use crate::action::{ResultData, ScreenshotResult};

use super::mouse_hover::get_frame_mouse_position;

impl ScreenshotResult {
    pub fn show(&self, ctx: &egui::Context, screenshot_rect: &Rect) {
        let frame_mouse_position = get_frame_mouse_position(ctx);

        for (i, result) in self.ocr_results.iter().enumerate() {
            let rect = result.get_ui_rect(ctx);
            let rect = rect.translate(screenshot_rect.left_top().to_vec2());
            let area = egui::Area::new(Id::new(format!("ScreenshotResult {}", i)))
                .current_pos(rect.left_top())
                .sense(Sense::click())
                .show(ctx, |ui| {
                    ui.set_width(rect.width());
                    ui.set_height(rect.height());

                    let color = if rect.contains(frame_mouse_position) {
                        show_ocr_info_window(ctx, &rect, &result);

                        Color32::GREEN
                    } else {
                        Color32::BLUE
                    };

                    if ctx.wants_pointer_input() {
                        let scroll_y = ctx.input(|state| state.raw_scroll_delta.y);

                        if scroll_y != 0.0 {
                            let id = Id::new("Scroll Y");

                            let value = ctx.data_mut(|map| {
                                let value = map.get_temp::<f32>(id).unwrap_or_default() + scroll_y;

                                map.insert_temp(id, value);

                                value
                            });
                            info!("Scroll Y: {}", value);
                        }
                    }
                    ui.painter()
                        .rect(rect, 0.0, Color32::TRANSPARENT, (1.0, color));
                });

            if area.response.clicked() {
                info!("Clicked");
            }
        }
    }
}

fn show_ocr_info_window(ctx: &egui::Context, rect: &Rect, result: &ResultData) {
    egui::Window::new("OCR Info")
        .current_pos(Pos2::new(rect.right(), rect.top()))
        .show(ctx, |ui| {
            ui.label(RichText::new(format!("{}", result.ocr)));
        });
}

impl ResultData {
    fn get_ui_rect(&self, ctx: &egui::Context) -> Rect {
        let zoom_factor = ctx.zoom_factor();

        let rect = Rect::from_min_size(
            Pos2::new(self.x as f32, self.y as f32),
            Vec2 {
                x: self.w as f32,
                y: self.h as f32,
            },
        );
        scale_rect(rect, 1.0 / zoom_factor)
    }
}

fn scale_rect(rect: Rect, scale_factor: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(rect.min.x * scale_factor, rect.min.y * scale_factor),
        Vec2 {
            x: rect.width() * scale_factor,
            y: rect.height() * scale_factor,
        },
    )
}

use crate::OcrApp;
use egui::{Color32, Context, Id, Order, Pos2, Rect, Vec2};
use enigo::{Enigo, Mouse, Settings as EnigoSettings};

impl OcrApp {
    pub fn update_mouse_passthrough(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(
            self.should_mouse_passthrough(ctx),
        ));
    }

    pub fn draw_mouse_position(&self, ctx: &egui::Context) {
        let color = if self.should_mouse_passthrough(ctx) {
            Color32::RED
        } else {
            Color32::GREEN
        };

        egui::Area::new(Id::new("Mouse Position")).show(ctx, |ui| {
            ui.painter()
                .circle_filled(get_frame_mouse_position(ctx), 10.0, color);
        });
    }

    fn should_mouse_passthrough(&self, ctx: &egui::Context) -> bool {
        self.settings.mouse_passthrough && is_mouse_over_background(ctx)
    }
}

pub fn is_mouse_over_background(ctx: &egui::Context) -> bool {
    if let Some(layer_id_at) = ctx.layer_id_at(get_frame_mouse_position(ctx)) {
        return layer_id_at.order == Order::Background;
    } else {
        return false;
    };
}

pub fn get_frame_mouse_position(ctx: &egui::Context) -> Pos2 {
    let frame_rect = get_frame_rect(ctx);

    let mouse_pos2 = get_mouse_position();

    let zoom_factor = ctx.zoom_factor();
    let mouse_pos2 = Pos2::new(mouse_pos2.x / zoom_factor, mouse_pos2.y / zoom_factor);
    let Vec2 { x, y } = mouse_pos2 - frame_rect.min;
    let mouse_pos2 = Pos2::new(x, y);
    mouse_pos2
}

pub fn get_frame_rect(ctx: &Context) -> Rect {
    let mut frame_rect: Rect = Rect::ZERO;
    ctx.input(|x| {
        frame_rect = x.viewport().inner_rect.unwrap_or(Rect::ZERO);
    });
    frame_rect
}

fn get_mouse_position() -> Pos2 {
    let enigo = Enigo::new(&EnigoSettings::default()).unwrap();
    let (x, y) = enigo.location().unwrap();

    Pos2::new(x as f32, y as f32)
}

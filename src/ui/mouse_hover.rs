use crate::OcrApp;
use anyhow::Result;
use egui::{Color32, Context, Id, Order, Pos2, Rect, Vec2};
use enigo::{Enigo, Mouse, Settings as EnigoSettings};

impl OcrApp {
    pub fn update_mouse_passthrough(&self, ctx: &Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(
            self.should_mouse_passthrough(ctx),
        ));
    }

    pub fn draw_mouse_position(&self, ctx: &Context) {
        let color = if self.should_mouse_passthrough(ctx) {
            Color32::RED
        } else {
            Color32::GREEN
        };

        egui::Area::new(Id::new("Mouse Position"))
            .order(Order::Debug)
            .show(ctx, |ui| {
                if let Ok(position) = get_frame_mouse_position(ctx) {
                    ui.painter().circle_filled(position, 10.0, color);
                }
            });
    }

    fn should_mouse_passthrough(&self, ctx: &Context) -> bool {
        self.settings.mouse_passthrough && is_mouse_over_background(ctx)
    }
}

pub fn is_mouse_over_background(ctx: &Context) -> bool {
    let Ok(position) = get_frame_mouse_position(ctx) else {
        return false;
    };
    if let Some(layer_id_at) = ctx.layer_id_at(position) {
        layer_id_at.order == Order::Background
    } else {
        false
    }
}

pub fn get_frame_mouse_position(ctx: &Context) -> Result<Pos2> {
    let frame_rect = get_frame_rect(ctx);

    let mouse_pos2 = get_mouse_position()?;

    let zoom_factor = ctx.zoom_factor();
    let mouse_pos2 = Pos2::new(mouse_pos2.x / zoom_factor, mouse_pos2.y / zoom_factor);
    let Vec2 { x, y } = mouse_pos2 - frame_rect.min;

    Ok(Pos2::new(x, y))
}

pub fn get_frame_rect(ctx: &Context) -> Rect {
    let mut frame_rect: Rect = Rect::ZERO;
    ctx.input(|x| {
        frame_rect = x.viewport().inner_rect.unwrap_or(Rect::ZERO);
    });
    frame_rect
}

fn get_mouse_position() -> Result<Pos2> {
    let enigo = Enigo::new(&EnigoSettings::default())?;
    let (x, y) = enigo.location()?;

    Ok(Pos2::new(x as f32, y as f32))
}

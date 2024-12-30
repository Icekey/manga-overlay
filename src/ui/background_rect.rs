use super::{channel_value::ChannelValue, mouse_hover::get_frame_rect, show_ui::ShowUi};
use crate::{
    action::{screenshot, ScreenshotParameter, ScreenshotResult},
    detect::comictextdetector::DETECT_STATE,
    ocr::{OcrBackend, OCR_STATE},
};

use egui::{Color32, ColorImage, Id, ImageData, Pos2, Rect, Sense, TextureOptions, Vec2};
use log::info;
use std::sync::Arc;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BackgroundRect {
    start_pos: Pos2,
    end_pos: Pos2,

    #[serde(skip)]
    pub drag_done: bool,

    channel_value: ChannelValue<ScreenshotResult>,
}

impl Default for BackgroundRect {
    fn default() -> Self {
        Self {
            start_pos: Default::default(),
            end_pos: Default::default(),
            drag_done: false,
            channel_value: Default::default(),
        }
    }
}

impl ShowUi for BackgroundRect {
    fn show(&mut self, ctx: &egui::Context) {
        self.channel_value.update();

        let frame_rect = get_frame_rect(ctx);
        let bg_response = draw_background(ctx, frame_rect);

        self.update_drag(&bg_response.response, ctx);

        self.channel_value.value.show(ctx, &self.get_rect());

        if let Some(capture_image) = &self.channel_value.value.capture_image {
            show_image_in_window(ctx, capture_image);
        }
    }
}

fn show_image_in_window(ctx: &egui::Context, capture_image: &image::DynamicImage) {
    egui::Window::new("Image").show(ctx, |ui| {
        let image = capture_image.clone();

        let mut screen_texture = ctx.load_texture(
            "screen",
            ImageData::Color(Arc::new(ColorImage::new(
                [image.width() as usize, image.height() as usize],
                Color32::TRANSPARENT,
            ))),
            TextureOptions::default(),
        );
        screen_texture.set(
            ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                &*image.clone().as_bytes(),
            ),
            TextureOptions::default(),
        );
        ui.add(
            egui::Image::new(&screen_texture)
                .fit_to_original_size(1.0 / ctx.zoom_factor()) // ERROR GONE
                .rounding(10.0),
        );
    });
}

impl BackgroundRect {
    fn update_drag(&mut self, response: &egui::Response, ctx: &egui::Context) {
        if response.drag_started() {
            if let Some(mpos) = response.interact_pointer_pos() {
                self.start_pos = mpos;
            }
        }

        self.drag_done = false;
        if response.dragged() || response.drag_stopped() {
            if let Some(mpos) = response.interact_pointer_pos() {
                self.end_pos = mpos;
                if response.drag_stopped() {
                    self.drag_done = true;
                    self.start_ocr(ctx);
                }
            }
        }
    }

    pub fn get_rect(&self) -> Rect {
        Rect::from_two_pos(self.start_pos, self.end_pos)
    }

    pub fn get_global_rect(&self, ctx: &egui::Context) -> Rect {
        let mut rect = self.get_rect();
        let zoom_factor = ctx.zoom_factor();
        let frame_rect = get_frame_rect(ctx);
        // let mut rect = rect.translate(Vec2::new(frame_rect.top(), frame_rect.left()));

        info!("rect: {:?}", rect);
        info!("frame_rect: {:?}", frame_rect);

        rect.set_top(rect.top() * zoom_factor);
        rect.set_left(rect.left() * zoom_factor);
        rect.set_right(rect.right() * zoom_factor);
        rect.set_bottom(rect.bottom() * zoom_factor);

        rect = rect.translate(Vec2::new(
            frame_rect.left() * zoom_factor,
            frame_rect.top() * zoom_factor,
        ));

        info!("rect2: {:?}", rect);
        // let rect = rect.scale_from_center(zoom_factor);

        rect
    }

    fn start_ocr(&self, ctx: &egui::Context) {
        if !self.drag_done {
            return;
        }
        let tx = self.channel_value.tx();
        let ocr_state = OCR_STATE.clone();
        let detect_state = DETECT_STATE.clone();

        let global_rect = self.get_global_rect(ctx);

        let screenshot_parameter = ScreenshotParameter {
            x: global_rect.min.x as i32,
            y: global_rect.min.y as i32,
            width: global_rect.width() as u32,
            height: global_rect.height() as u32,
            detect_boxes: true,
            //TODO: Backend auswahl
            backends: vec![OcrBackend::MangaOcr],
            ..Default::default()
        };

        tokio::spawn(async move {
            info!("Start screenshot");
            let screenshot = screenshot(screenshot_parameter, &ocr_state, &detect_state)
                .await
                .unwrap();

            info!("Stop screenshot");

            let _ = tx.send(screenshot).await;
        });
    }
}

fn draw_background(ctx: &egui::Context, frame_rect: Rect) -> egui::InnerResponse<()> {
    egui::Area::new(Id::new("Background"))
        .order(egui::Order::Background)
        .sense(Sense::drag())
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(frame_rect.width());
            ui.set_height(frame_rect.height());
        })
}

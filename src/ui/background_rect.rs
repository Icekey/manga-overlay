use super::{mouse_hover::get_frame_rect, screenshot_result_ui::scale_rect, settings::AppSettings};
use crate::action::{run_ocr, ScreenshotParameter, ScreenshotResult};
use crate::ocr::OcrBackend::MangaOcr;
use crate::ui::event::Event::UpdateScreenshotResult;
use crate::ui::event::EventHandler;
use crate::ui::shutdown::TASK_TRACKER;
use eframe::epaint::StrokeKind;
use egui::{Color32, Context, Id, Pos2, Rect, Sense, TextureHandle, Vec2};
use log::info;
use std::time::Duration;
use tokio::time::Instant;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct BackgroundRect {
    start_pos: Pos2,
    end_pos: Pos2,

    pub screenshot_result: ScreenshotResult,
    #[serde(skip)]
    pub hide_ocr_rects: bool,

    #[serde(skip)]
    pub start_ocr_at: Option<Instant>,
    #[serde(skip)]
    last_ocr_rect_hover_at: Option<Instant>,

    #[serde(skip)]
    pub capture_image_handle: Option<TextureHandle>,
    #[serde(skip)]
    pub debug_image_handle: Option<TextureHandle>,
}

pub fn start_ocr_id() -> Id {
    Id::new("start_ocr")
}

fn is_start_ocr(ctx: &egui::Context) -> bool {
    ctx.data_mut(|map| {
        let id = start_ocr_id();
        let value = map.get_temp(id).unwrap_or(false);
        map.insert_temp(id, false);
        value
    })
}

impl BackgroundRect {
    pub fn show(&mut self, ctx: &egui::Context, settings: &AppSettings) {
        self.check_start_ocr(ctx, settings);

        let bg_response = self.draw_background(ctx);

        if !settings.mouse_passthrough && self.update_drag(&bg_response.response, ctx.zoom_factor())
        {
            self.start_ocr_at = Some(Instant::now());
        }

        if bg_response.response.drag_started() {
            self.screenshot_result = Default::default();
        }

        if settings.show_capture_image {
            show_image_in_window(ctx, "Capture Image", self.capture_image_handle.clone());
        }
        if settings.show_debug_image {
            show_image_in_window(ctx, "Debug Image", self.debug_image_handle.clone());
        }
    }

    fn check_start_ocr(&mut self, ctx: &Context, settings: &AppSettings) {
        if self.hide_ocr_rects {
            //Rect are hidden => screenshot can be taken
            self.start_ocr(ctx, settings);
            self.hide_ocr_rects = false;
        }

        if is_start_ocr(ctx) || self.should_auto_restart(settings) {
            self.start_ocr_at = None;
            self.hide_ocr_rects = true;
        }
    }

    fn should_auto_restart(&mut self, settings: &AppSettings) -> bool {
        if let Some(instant) = self.start_ocr_at {
            let not_hovering = self.last_ocr_rect_hover_at.map_or(true, |x| {
                x.elapsed() >= Duration::from_millis(settings.hover_delay_ms)
            });

            let elapsed = instant.elapsed();
            return elapsed >= Duration::from_millis(settings.auto_restart_delay_ms)
                && not_hovering;
        }
        false
    }
}

fn show_image_in_window(ctx: &egui::Context, title: &str, texture: Option<TextureHandle>) {
    egui::Window::new(title).show(ctx, |ui| {
        if let Some(texture) = texture {
            ui.add(
                egui::Image::new(&texture)
                    .shrink_to_fit()
                    .corner_radius(10.0),
            );
        } else {
            ui.label("No Image");
        }
    });
}

impl BackgroundRect {
    fn update_drag(&mut self, response: &egui::Response, zoom_factor: f32) -> bool {
        if response.drag_started() {
            if let Some(mpos) = response.interact_pointer_pos() {
                self.start_pos = mpos * zoom_factor;
            }
        }

        if response.dragged() || response.drag_stopped() {
            if let Some(mpos) = response.interact_pointer_pos() {
                self.end_pos = mpos * zoom_factor;
                if response.drag_stopped() {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_unscaled_rect(&self) -> Rect {
        Rect::from_two_pos(self.start_pos, self.end_pos)
    }

    pub fn get_global_rect(&self, ctx: &egui::Context) -> Rect {
        let mut rect = self.get_unscaled_rect();
        let frame_rect = get_frame_rect(ctx);

        let zoom_factor = ctx.zoom_factor();
        rect = rect.translate(Vec2::new(
            frame_rect.left() * zoom_factor,
            frame_rect.top() * zoom_factor,
        ));

        rect
    }

    fn start_ocr(&self, ctx: &egui::Context, settings: &AppSettings) {
        let global_rect = self.get_global_rect(ctx);

        let screenshot_parameter = ScreenshotParameter {
            x: global_rect.min.x as i32,
            y: global_rect.min.y as i32,
            width: global_rect.width() as u32,
            height: global_rect.height() as u32,
            detect_boxes: settings.detect_boxes,
            full_capture_ocr: !settings.detect_boxes,
            backends: vec![MangaOcr],
            threshold: settings.threshold,
        };

        let image = screenshot_parameter.get_screenshot().unwrap();

        let ctx = ctx.clone();
        TASK_TRACKER.spawn(async move {
            info!("Start ocr");
            let screenshot = run_ocr(screenshot_parameter, image).await.unwrap();
            info!("Start ocr done");

            ctx.emit(UpdateScreenshotResult(screenshot));
        });
    }

    fn draw_background(&mut self, ctx: &egui::Context) -> egui::InnerResponse<()> {
        let frame_rect = get_frame_rect(ctx);
        let rect = self.get_unscaled_rect();

        let rect = scale_rect(rect, 1.0 / ctx.zoom_factor());

        if !self.hide_ocr_rects && self.screenshot_result.show(ctx, &rect) {
            self.last_ocr_rect_hover_at = Some(Instant::now());
        }

        egui::Area::new(Id::new("Background"))
            .order(egui::Order::Background)
            .sense(Sense::drag())
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(frame_rect.width());
                ui.set_height(frame_rect.height());

                ui.painter().rect(
                    rect,
                    0.0,
                    Color32::TRANSPARENT,
                    (1.0, Color32::RED),
                    StrokeKind::Middle,
                );
            })
    }
}

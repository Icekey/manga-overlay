use super::{mouse_hover::get_frame_rect, screenshot_result_ui::scale_rect, settings::AppSettings};
use crate::action::{OcrPipeline, ScreenshotParameter, ScreenshotResult, run_ocr};
use crate::event::event::{Event, emit_event, is_minimized};
use crate::ui::shutdown::TASK_TRACKER;
use eframe::epaint::StrokeKind;
use egui::{Color32, Context, Id, Pos2, Rect, Sense, Vec2};
use image::DynamicImage;
use log::warn;
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
}

pub fn start_ocr_id() -> Id {
    Id::new("start_ocr")
}

fn is_start_ocr(ctx: &Context) -> bool {
    ctx.data_mut(|map| {
        let id = start_ocr_id();
        let value = map.get_temp(id).unwrap_or(false);
        map.insert_temp(id, false);
        value
    })
}

impl BackgroundRect {
    pub fn show(&mut self, ctx: &Context, settings: &AppSettings) {
        self.check_start_ocr(ctx, settings);

        let bg_response =
            self.draw_background(ctx, settings.mouse_passthrough, settings.clear_color);

        if !settings.mouse_passthrough
            && self.update_drag(settings, &bg_response.response, ctx.zoom_factor())
        {
            self.start_ocr_at = Some(Instant::now());
        }

        if bg_response.response.drag_started() {
            self.screenshot_result = Default::default();
        }

        if bg_response.response.dragged() {
            ctx.data_mut(|x| x.insert_temp(Id::new("ocr_is_cancelled"), true));
        }
    }

    fn check_start_ocr(&mut self, ctx: &Context, settings: &AppSettings) {
        if self.hide_ocr_rects {
            //Rects are hidden => screenshot can be taken
            self.start_ocr(ctx, settings);
            self.hide_ocr_rects = false;
        }

        if is_start_ocr(ctx) || self.should_auto_restart(ctx, settings) {
            self.start_ocr_at = None;
            self.hide_ocr_rects = true;
        }
    }

    fn should_auto_restart(&mut self, ctx: &Context, settings: &AppSettings) -> bool {
        if is_minimized(ctx) {
            return false;
        }

        let clicked_result_id = Id::new("clicked_result_pos");
        if ctx.data(|map| map.get_temp::<Pos2>(clicked_result_id).is_some()) {
            //ocr result rect is selected
            return false;
        }

        if let Some(instant) = self.start_ocr_at {
            let not_hovering = self.last_ocr_rect_hover_at.map_or(true, |x| {
                x.elapsed() >= Duration::from_millis(settings.hover_delay_ms)
            });

            let elapsed = instant.elapsed();
            return elapsed > Duration::from_millis(0) && not_hovering;
        }
        false
    }
}

impl BackgroundRect {
    fn update_drag(
        &mut self,
        settings: &AppSettings,
        response: &egui::Response,
        zoom_factor: f32,
    ) -> bool {
        if response.drag_started() {
            if let Some(mpos) = response.interact_pointer_pos() {
                self.start_pos = mpos * zoom_factor;
            }
        }

        if response.dragged() {
            if let Some(mpos) = response.interact_pointer_pos() {
                self.end_pos = mpos * zoom_factor;
            }
        }

        if response.drag_stopped() {
            if settings.quick_area_pick_mode {
                emit_event(Event::UpdateMousePassthrough(true));
            }
            return true;
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

    fn start_ocr(&self, ctx: &Context, settings: &AppSettings) {
        let global_rect = self.get_global_rect(ctx);

        let screenshot_parameter = ScreenshotParameter {
            x: global_rect.min.x as i32,
            y: global_rect.min.y as i32,
            width: global_rect.width() as u32,
            height: global_rect.height() as u32,
            pipeline: OcrPipeline(settings.get_current_pipeline().items.clone()),
        };

        let Ok(image) = screenshot_parameter.get_screenshot() else {
            warn!("screenshot_parameter get screenshot failed");
            return;
        };

        if are_inputs_unchanged(&ctx, screenshot_parameter.clone(), image.clone()) {
            emit_event(Event::ResetOcrStartTime);
            return;
        }
        ctx.data_mut(|x| x.insert_temp(Id::new("ocr_is_cancelled"), false));

        let auto_restart = settings.auto_restart_ocr;
        TASK_TRACKER.spawn(async move {
            run_ocr(image, screenshot_parameter.pipeline).await;

            if auto_restart {
                emit_event(Event::ResetOcrStartTime);
            }
        });
    }

    fn draw_background(
        &mut self,
        ctx: &Context,
        mouse_passthrough: bool,
        clear_color: Color32,
    ) -> egui::InnerResponse<()> {
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
                let width = frame_rect.width();
                ui.set_width(width);
                let height = frame_rect.height();
                ui.set_height(height);

                if !mouse_passthrough {
                    //Draw Background Rect
                    let top_rect = Rect::from_two_pos(Pos2::ZERO, Pos2::new(width, rect.min.y));
                    let left_rect = Rect::from_two_pos(
                        Pos2::new(0.0, rect.min.y),
                        Pos2::new(rect.min.x, rect.max.y),
                    );
                    let right_rect = Rect::from_two_pos(
                        Pos2::new(rect.max.x, rect.min.y),
                        Pos2::new(frame_rect.max.x, rect.max.y),
                    );
                    let bottom_rect = Rect::from_two_pos(
                        Pos2::new(0.0, rect.max.y),
                        Pos2::new(frame_rect.max.x, frame_rect.max.y),
                    );
                    ui.painter().rect_filled(top_rect, 0.0, clear_color);
                    ui.painter().rect_filled(left_rect, 0.0, clear_color);
                    ui.painter().rect_filled(right_rect, 0.0, clear_color);
                    ui.painter().rect_filled(bottom_rect, 0.0, clear_color);
                }

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

fn are_inputs_unchanged(
    ctx: &Context,
    parameter: ScreenshotParameter,
    image: DynamicImage,
) -> bool {
    let param_id = Id::new("last_parameter");
    let image_id = Id::new("last_image");

    // Check if both parameter and image are unchanged
    let unchanged = ctx.data_mut(|x| {
        if let Some(last_parameter) = x.remove_temp::<ScreenshotParameter>(param_id)
            && last_parameter == parameter
            && let Some(last_image) = x.remove_temp::<DynamicImage>(image_id)
            && last_image.eq(&image)
        {
            true
        } else {
            false
        }
    });

    // Store current parameter and image for next comparison
    ctx.data_mut(|x| {
        x.insert_temp(param_id, parameter);
        x.insert_temp(image_id, image);
    });

    unchanged
}

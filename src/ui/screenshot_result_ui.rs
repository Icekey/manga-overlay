use super::mouse_hover::get_frame_mouse_position;
use crate::action::{self, ResultData, ScreenshotResult, get_translation};
use crate::event::event::Event::UpdateOcrResult;
use crate::event::event::emit_event;
use crate::ocr::BackendResult;
use crate::ui::shutdown::TASK_TRACKER;
use eframe::epaint::StrokeKind;
use egui::{Align2, Color32, Context, Id, Pos2, Rect, RichText, Sense, Vec2, Window};
use itertools::Itertools;
use std::time::{Duration, Instant};

impl ScreenshotResult {
    pub fn show(&mut self, ctx: &egui::Context, screenshot_rect: &Rect) -> bool {
        self.update_translation(ctx);

        let frame_mouse_position = get_frame_mouse_position(ctx).unwrap_or_default();
        let mut area_hovered = false;

        let clicked_result_id = Id::new("clicked_result_pos");
        let clicked_result_pos = ctx.data(|x| x.get_temp::<Pos2>(clicked_result_id));

        for (i, result) in self.ocr_results.iter().enumerate() {
            let rect = result.get_ui_rect(ctx);
            let rect = rect.translate(screenshot_rect.left_top().to_vec2());
            let rect_is_clicked = clicked_result_pos.filter(|x| rect.contains(*x)).is_some();
            let area = egui::Area::new(Id::new(format!("ScreenshotResult {} {}", i, result.ocr)))
                .current_pos(rect.left_top())
                .sense(Sense::click())
                .show(ctx, |ui| {
                    ui.set_width(rect.width());
                    ui.set_height(rect.height());
                    let contains = rect.contains(frame_mouse_position);

                    let is_active = contains || rect_is_clicked;
                    if is_active {
                        show_ocr_info_window(ctx, &rect, result, i, rect_is_clicked);
                    }

                    let color = if is_active {
                        Color32::GREEN
                    } else {
                        Color32::BLUE
                    };

                    ui.painter().rect(
                        rect,
                        0.0,
                        Color32::TRANSPARENT,
                        (1.0, color),
                        StrokeKind::Middle,
                    );
                });

            if area.response.clicked() {
                if result.translation.is_empty() {
                    fetch_translation(&result.ocr, i, ctx);
                } else {
                    set_translation_visible(ctx, !is_translation_visible(ctx));
                }
            }

            if area.response.secondary_clicked() {
                if rect_is_clicked {
                    ctx.data_mut(|x| x.remove_temp::<Pos2>(clicked_result_id));
                } else {
                    ctx.data_mut(|x| x.insert_temp(clicked_result_id, frame_mouse_position));
                }
            }

            if area.response.hovered() {
                area_hovered = true;
            }
        }

        update_scroll_y_offset(ctx, area_hovered);
        area_hovered
    }

    fn update_translation(&mut self, ctx: &egui::Context) {
        let translation_id = Id::new("translation");
        let update_translation =
            ctx.data_mut(|map| map.get_temp::<TranslationUpdate>(translation_id));
        if let Some(update) = update_translation {
            self.ocr_results[update.index].translation = update.translation;
            ctx.data_mut(|x| x.remove_temp::<TranslationUpdate>(translation_id));

            set_translation_visible(ctx, true);
        }
    }
}

fn is_translation_visible(ctx: &egui::Context) -> bool {
    ctx.data(|map| map.get_temp::<bool>(Id::new("is_translation_visible")))
        .unwrap_or_default()
}

fn set_translation_visible(ctx: &egui::Context, is_visible: bool) {
    ctx.data_mut(|map| map.insert_temp::<bool>(Id::new("is_translation_visible"), is_visible));
}

fn fetch_translation(ocr: &str, index: usize, ctx: &egui::Context) {
    let ocr = ocr.to_owned();
    let ctx = ctx.clone();
    tokio::spawn(async move {
        let translation = get_translation(&ocr).await;
        ctx.data_mut(|x| {
            x.insert_temp(
                Id::new("translation"),
                TranslationUpdate { index, translation },
            );
        });
    });
}

#[derive(Clone, Default)]
struct TranslationUpdate {
    index: usize,
    translation: String,
}

fn update_scroll_y_offset(ctx: &egui::Context, area_hovered: bool) {
    let scroll_y_id = Id::new("Scroll Y");

    // Reset the scroll offset when the area is hovered
    if is_area_hover_start(ctx, area_hovered) {
        ctx.data_mut(|map| map.insert_temp(scroll_y_id, 0));
    }

    if !ctx.wants_pointer_input() {
        return;
    }

    let scroll_y = ctx.input(|state| state.raw_scroll_delta.y);
    if scroll_y == 0.0 {
        return;
    }

    let offset = if scroll_y > 0.0 { -1 } else { 1 };
    ctx.data_mut(|map| {
        let value = map.get_temp::<i32>(scroll_y_id).unwrap_or_default() + offset;

        map.insert_temp(scroll_y_id, value);
    });
}

fn is_area_hover_start(ctx: &egui::Context, area_hovered: bool) -> bool {
    let area_hovered_id = Id::new("area_hovered");
    let old_area_hovered = ctx
        .data(|mem| mem.get_temp::<bool>(area_hovered_id))
        .unwrap_or_default();

    ctx.data_mut(|map| map.insert_temp(area_hovered_id, area_hovered));
    !old_area_hovered && area_hovered
}

fn show_ocr_info_window(
    ctx: &egui::Context,
    rect: &Rect,
    result: &ResultData,
    index: usize,
    rect_is_clicked: bool,
) {
    let window = create_info_window(ctx, rect, result, index);

    window.show(ctx, |ui| {
        if !result.translation.is_empty() && is_translation_visible(ctx) {
            ui.label(get_info_text(&result.translation));
            ui.separator();
        }

        if rect_is_clicked {
            let selected_kanji_id = Id::new("selected_kanji_id");
            let selected_kanji_index: usize = ui
                .data(|map| map.get_temp(selected_kanji_id))
                .unwrap_or_default();

            let old_item_spacing = ui.spacing_mut().item_spacing;
            ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);

            let ocr_chars = result.ocr.chars().collect_vec();

            ui.horizontal_wrapped(|ui| {
                for (index, ocr_char) in ocr_chars.iter().enumerate() {
                    let mut text = RichText::new(ocr_char.to_string())
                        .underline()
                        .size(TEXT_SIZE);

                    if index == selected_kanji_index {
                        text = text.color(Color32::RED);
                    }

                    let response = ui.label(text);

                    if response.clicked() {
                        ui.data_mut(|map| map.insert_temp(selected_kanji_id, index));
                    }
                }
            });
            ui.spacing_mut().item_spacing = old_item_spacing;

            match &result.backend_result {
                BackendResult::MangaOcr(top) => {
                    ui.separator();

                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);

                    if let Some(kanji_result) = top.get(selected_kanji_index) {
                        ui.horizontal_wrapped(|ui| {
                            for kanji_conf in kanji_result.iter() {
                                let kanji = kanji_conf.kanji.clone();
                                let mut text = RichText::new(kanji.clone()).size(TEXT_SIZE);

                                if let Some(selected_char) = ocr_chars.get(selected_kanji_index)
                                    && kanji_conf.kanji.eq(&selected_char.to_string())
                                {
                                    text = text.color(Color32::RED);
                                };

                                let response1 = ui.label(text);

                                if response1.clicked() {
                                    let updated_ocr = replace_nth_char_safe(
                                        &result.ocr,
                                        selected_kanji_index,
                                        kanji.chars().next().unwrap(),
                                    );

                                    emit_event(UpdateOcrResult(index, updated_ocr))
                                }
                            }
                        });
                    }
                }
                BackendResult::Unknown => {}
            }
        } else {
            let id = Id::new("Scroll Y");
            let index = ui.data(|map| map.get_temp(id)).unwrap_or_default();
            let selected_jpn_data = result.get_jpn_data_with_info_by_index(index);
            for jpn in &result.jpn {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
                ui.horizontal_wrapped(|ui| {
                    for jpn_data in jpn {
                        let kanji = jpn_data.get_kanji();
                        let mut text = get_info_text(&kanji);
                        if jpn_data.has_kanji_data() {
                            text = text.underline();
                        }
                        if selected_jpn_data == Some(jpn_data) {
                            text = text.color(Color32::RED);
                        }
                        ui.label(text);
                    }
                });
            }

            if let Some(info) = selected_jpn_data {
                ui.separator();
                show_jpn_data_info(ui, info);
                update_kanji_statistic(ui, info);
            }
        }
    });
}

fn replace_nth_char_safe(s: &str, idx: usize, newchar: char) -> String {
    s.chars()
        .enumerate()
        .map(|(i, c)| if i == idx { newchar } else { c })
        .collect()
}

fn create_info_window<'a>(
    ctx: &'a Context,
    rect: &'a Rect,
    result: &'a ResultData,
    index: usize,
) -> Window<'a> {
    let right_side = rect.min.x > ctx.screen_rect().width() * 2.0 / 3.0;

    let (pivot, default_pos_x) = if right_side {
        (Align2::RIGHT_TOP, rect.left() - 3.0)
    } else {
        (Align2::LEFT_TOP, rect.right() + 3.0)
    };

    let last_ocr_text_id = Id::new("last_ocr_text_id");
    let changed_text = ctx.data_mut(|x| {
        let last_text: String = x.get_temp(last_ocr_text_id).unwrap_or_default();

        x.insert_temp(last_ocr_text_id, result.ocr.clone());

        last_text != result.ocr
    });

    let default_pos = Pos2::new(default_pos_x, rect.top());
    let mut window = egui::Window::new(format!("OCR Info {}", index))
        .title_bar(false)
        .pivot(pivot)
        .default_pos(default_pos.clone())
        .default_width(500.0);

    if changed_text {
        window = window.current_pos(default_pos);
    }
    window
}

pub fn show_jpn_data_info(ui: &mut egui::Ui, info: &crate::jpn::JpnData) {
    for info_row in info.get_info_rows() {
        ui.label(get_info_text(info_row));
    }
}

fn update_kanji_statistic(ui: &mut egui::Ui, info: &crate::jpn::JpnData) {
    let id = Id::new("show_kanji_timer");
    let kanji_timer = ui.data(|x| x.get_temp::<KanjiStatisticTimer>(id));

    if let Some(mut timer) = kanji_timer {
        if !timer.statistic_updated && timer.timestamp.elapsed() >= Duration::from_millis(500) {
            timer.statistic_updated = true;
            ui.data_mut(|x| x.insert_temp(id, timer));
            let kanji = info.get_kanji();

            TASK_TRACKER.spawn(async move {
                let _ = action::increment_kanji_statistic(&kanji);
            });
            return;
        }
        if timer.kanji == info.get_kanji() {
            return;
        }
    }
    ui.data_mut(|x| x.insert_temp(id, KanjiStatisticTimer::new(info.get_kanji())));
}

#[derive(Clone, Debug)]
struct KanjiStatisticTimer {
    kanji: String,
    timestamp: Instant,
    statistic_updated: bool,
}

impl KanjiStatisticTimer {
    fn new(kanji: String) -> Self {
        let timestamp = Instant::now();
        Self {
            kanji,
            timestamp,
            statistic_updated: false,
        }
    }
}

const TEXT_SIZE: f32 = 20.0;

fn get_info_text(text: impl Into<String>) -> RichText {
    RichText::new(text).size(TEXT_SIZE)
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

pub fn scale_rect(rect: Rect, scale_factor: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(rect.min.x * scale_factor, rect.min.y * scale_factor),
        Vec2 {
            x: rect.width() * scale_factor,
            y: rect.height() * scale_factor,
        },
    )
}

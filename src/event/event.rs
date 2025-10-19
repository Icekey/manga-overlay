use crate::OcrApp;
use crate::action::ScreenshotResult;
use crate::database::{HistoryData, KanjiStatistic};
use crate::jpn::{JpnData, get_jpn_data};
use crate::ocr::BackendResult;
use crate::ui::image_display::ImageWrapper;
use crate::ui::settings::{Backend, BackendStatus};
use crate::ui::shutdown::TASK_TRACKER;
use crate::ui::update_queue::enqueue_update;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{Context, Id, Memory};
use image::{DynamicImage, EncodableLayout};
use log::debug;
use std::cmp::max;
use std::ops::Add;
use std::time::Duration;
use tokio::time::Instant;

pub fn update_ocr_result(state: &mut OcrApp, index: usize, ocr: String) {
    if let Some(result_data) = state
        .background_rect
        .screenshot_result
        .ocr_results
        .get_mut(index)
        && result_data.ocr != ocr
    {
        result_data.translation = "".to_string();
        result_data.ocr = ocr.to_string();

        TASK_TRACKER.spawn(async move {
            let jpn = get_jpn_data(&ocr).await;

            enqueue_update(move |_, app| update_jpn_data(app, index, jpn));
        });
    }
}

pub fn update_jpn_data(state: &mut OcrApp, index: usize, jpn_data: Vec<Vec<JpnData>>) {
    if let Some(result_data) = state
        .background_rect
        .screenshot_result
        .ocr_results
        .get_mut(index)
    {
        result_data.jpn = jpn_data;
    }
}

pub fn is_minimized(ctx: &Context) -> bool {
    ctx.input(|i| i.viewport().minimized).unwrap_or_default()
}

pub fn update_screenshot_result(ctx: &Context, state: &mut OcrApp, data: ScreenshotResult) {
    if ctx
        .data(|x| x.get_temp(Id::new("ocr_is_cancelled")))
        .unwrap_or(false)
    {
        return;
    }

    for x in data.ocr_results.iter().map(|x| &x.backend_result) {
        match x {
            BackendResult::MangaOcr(top) => {
                for i in top {
                    for j in i {
                        print!("{}", j.kanji)
                    }
                    println!();
                }
            }
            BackendResult::Unknown => {}
        }
    }

    let background_rect = &mut state.background_rect;
    let settings = &state.settings;
    if settings.auto_restart_ocr {
        //Restart OCR
        reset_ocr_start_time();
    }

    background_rect.screenshot_result = data;
}

pub fn update_history_data(state: &mut OcrApp, data: Vec<HistoryData>) {
    state.history.history_data = data;
}

pub fn update_kanji_statistic(ctx: &Context, state: &mut OcrApp, data: Vec<KanjiStatistic>) {
    state.kanji_statistic.kanji_statistic = data;
    if state.kanji_statistic.selected_kanji_index.is_none() {
        state
            .kanji_statistic
            .update_selected_kanji_statistic(0, ctx);
    }
}

pub fn update_selected_jpn_data(state: &mut OcrApp, data: JpnData) {
    state.kanji_statistic.selected_jpn_data = data;
}

pub fn update_backend_status(backend: Backend, backend_status: BackendStatus) {
    enqueue_update(move |ctx: &Context, _| {
        debug!(
            "Update Backend '{:#?}' to Status '{:#?}'",
            backend, backend_status
        );
        backend.set_status(ctx, backend_status);
    });
}

pub fn reset_ui(ctx: &Context, state: &mut OcrApp) {
    ctx.memory_mut(|x| *x = Memory::default());
    state.reset();
}

pub fn reset_ocr_start_time() {
    enqueue_update(move |_, state: &mut OcrApp| {
        let background_rect = &mut state.background_rect;
        let settings = &state.settings;
        background_rect.start_ocr_at =
            Some(Instant::now().add(Duration::from_millis(settings.auto_restart_delay_ms)));
    });
}

pub fn update_image_display(
    ctx: &Context,
    state: &mut OcrApp,
    index: usize,
    max_index: usize,
    label: String,
    image: Option<DynamicImage>,
) {
    let texture = create_texture(ctx, image.as_ref(), &format!("debug_image_{index}"));

    let image_handles = &mut state.settings.debug_images.image_handles;
    let vec_size = max(index + 1, max_index);
    image_handles.resize(vec_size, ImageWrapper::default());

    let wrapper = &mut image_handles[index];
    wrapper.label = label;
    wrapper.image_handle = texture;
}

fn create_texture(
    ctx: &Context,
    image: Option<&DynamicImage>,
    name: &str,
) -> Option<TextureHandle> {
    image.map(|image| {
        ctx.load_texture(
            name,
            match image {
                DynamicImage::ImageLuma8(x) => {
                    ColorImage::from_gray([x.width() as usize, x.height() as usize], x.as_bytes())
                }
                x => ColorImage::from_rgba_unmultiplied(
                    [x.width() as usize, x.height() as usize],
                    x.as_bytes(),
                ),
            },
            TextureOptions::default(),
        )
    })
}

pub fn remove_pipeline_step(state: &mut OcrApp, index: usize) {
    let current_pipeline = state.settings.get_current_pipeline_mut();
    if current_pipeline.items.len() > index {
        current_pipeline.items.remove(index);
    }
    if state.settings.debug_images.image_handles.len() > index {
        state.settings.debug_images.image_handles.remove(index);
    }
}

pub fn update_mouse_passthrough(mouse_passthrough: bool) {
    enqueue_update(move |_, app| {
        update_mouse_passthrough_internal(app, mouse_passthrough);
    });
}

pub fn toggle_mouse_passthrough() {
    enqueue_update(move |_, app| {
        update_mouse_passthrough_internal(app, !app.settings.mouse_passthrough);
    });
}

fn update_mouse_passthrough_internal(state: &mut OcrApp, mouse_passthrough: bool) {
    if mouse_passthrough {
        state.settings.quick_area_pick_mode = false;
    }
    state.settings.mouse_passthrough = mouse_passthrough;
}

pub fn update_decorations(decorations: bool) {
    enqueue_update(move |ctx, app| {
        update_decorations_internal(ctx, app, decorations);
    });
}

pub fn toggle_decorations() {
    enqueue_update(move |ctx, app| {
        update_decorations_internal(ctx, app, !app.settings.decorations);
    });
}

pub fn update_decorations_internal(ctx: &Context, app: &mut OcrApp, decorations: bool) {
    app.settings.decorations = decorations;
    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(decorations));
}

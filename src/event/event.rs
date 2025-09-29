use crate::OcrApp;
use crate::action::ScreenshotResult;
use crate::database::{HistoryData, KanjiStatistic};
use crate::event::event::Event::UpdateJpnData;
use crate::jpn::{JpnData, get_jpn_data};
use crate::ocr::BackendResult;
use crate::ui::image_display::ImageWrapper;
use crate::ui::settings::{Backend, BackendStatus};
use crate::ui::shutdown::TASK_TRACKER;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{Context, Id, Memory};
use global_hotkey::hotkey::HotKey;
use image::{DynamicImage, EncodableLayout};
use log::debug;
use std::cmp::max;
use std::ops::Add;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use strum::EnumIter;
use subenum::subenum;
use tokio::time::Instant;

#[subenum(ShortcutEvent(derive(serde::Deserialize, serde::Serialize, EnumIter)))]
#[derive(PartialEq, Debug, Clone)]
pub enum Event {
    UpdateScreenshotResult(ScreenshotResult),
    UpdateHistoryData(Vec<HistoryData>),
    UpdateKanjiStatistic(Vec<KanjiStatistic>),
    UpdateSelectedJpnData(JpnData),
    UpdateBackendStatus(Backend, BackendStatus),
    ResetUi,
    ResetOcrStartTime,
    UpdateImageDisplay(usize, usize, String, Option<DynamicImage>),
    RemovePipelineStep(usize),
    UpdateDecorations(bool),
    #[subenum(ShortcutEvent)]
    ToggleDecorations,
    UpdateMousePassthrough(bool),
    #[subenum(ShortcutEvent)]
    ToggleMousePassthrough,
    UpdateShortcut(ShortcutEvent, HotKey),
    #[subenum(ShortcutEvent)]
    ToggleMinimized,
    #[subenum(ShortcutEvent)]
    QuickAreaPickMode,
    UpdateOcrResult(usize, String),
    UpdateJpnData(usize, Vec<Vec<JpnData>>),
}

impl Event {
    pub fn handle_event(self, ctx: &Context, state: &mut OcrApp) {
        match self {
            Event::UpdateScreenshotResult(data) => update_screenshot_result(ctx, state, data),
            Event::UpdateHistoryData(data) => update_history_data(state, data),
            Event::UpdateKanjiStatistic(data) => update_kanji_statistic(ctx, state, data),
            Event::UpdateSelectedJpnData(data) => update_selected_jpn_data(state, data),
            Event::UpdateBackendStatus(backend, status) => {
                update_backend_status(ctx, backend, status)
            }
            Event::ResetUi => reset_ui(ctx, state),
            Event::ResetOcrStartTime => reset_ocr_start_time(state),
            Event::UpdateImageDisplay(index, max_index, label, image) => {
                update_image_display(ctx, state, index, max_index, label, image)
            }
            Event::RemovePipelineStep(index) => remove_pipeline_step(state, index),
            Event::UpdateDecorations(data) => update_decorations(ctx, state, data),
            Event::ToggleDecorations => update_decorations(ctx, state, !state.settings.decorations),
            Event::UpdateMousePassthrough(data) => update_mouse_passthrough(state, data),
            Event::ToggleMousePassthrough => {
                update_mouse_passthrough(state, !state.settings.mouse_passthrough);
            }
            Event::UpdateShortcut(event, hotkey) => {
                state.settings.shortcut.update_hotkey(event, hotkey);
            }
            Event::ToggleMinimized => {
                let is_minimized = is_minimized(ctx);
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(!is_minimized));
            }
            Event::QuickAreaPickMode => {
                update_mouse_passthrough(state, false);
                state.settings.quick_area_pick_mode = !state.settings.quick_area_pick_mode
            }
            Event::UpdateOcrResult(index, ocr) => {
                update_ocr_result(state, index, ocr);
            }
            Event::UpdateJpnData(index, jpn_data) => update_jpn_data(state, index, jpn_data),
        }
    }
}

fn update_ocr_result(state: &mut OcrApp, index: usize, ocr: String) {
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
            emit_event(UpdateJpnData(index, jpn));
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

fn update_screenshot_result(ctx: &Context, state: &mut OcrApp, data: ScreenshotResult) {
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
        emit_event(Event::ResetOcrStartTime);
    }

    background_rect.screenshot_result = data;
}

fn update_history_data(state: &mut OcrApp, data: Vec<HistoryData>) {
    state.history.history_data = data;
}

fn update_kanji_statistic(ctx: &Context, state: &mut OcrApp, data: Vec<KanjiStatistic>) {
    state.kanji_statistic.kanji_statistic = data;
    if state.kanji_statistic.selected_kanji_index.is_none() {
        state
            .kanji_statistic
            .update_selected_kanji_statistic(0, ctx);
    }
}

fn update_selected_jpn_data(state: &mut OcrApp, data: JpnData) {
    state.kanji_statistic.selected_jpn_data = data;
}

fn update_backend_status(ctx: &Context, backend: Backend, backend_status: BackendStatus) {
    debug!(
        "Update Backend '{:#?}' to Status '{:#?}'",
        backend, backend_status
    );
    backend.set_status(ctx, backend_status);
}

fn reset_ui(ctx: &Context, state: &mut OcrApp) {
    ctx.memory_mut(|x| *x = Memory::default());
    state.reset();
}

fn reset_ocr_start_time(state: &mut OcrApp) {
    let background_rect = &mut state.background_rect;
    let settings = &state.settings;
    background_rect.start_ocr_at =
        Some(Instant::now().add(Duration::from_millis(settings.auto_restart_delay_ms)));
}

fn update_image_display(
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

fn remove_pipeline_step(state: &mut OcrApp, index: usize) {
    let current_pipeline = state.settings.get_current_pipeline_mut();
    if current_pipeline.items.len() > index {
        current_pipeline.items.remove(index);
    }
    if state.settings.debug_images.image_handles.len() > index {
        state.settings.debug_images.image_handles.remove(index);
    }
}

fn update_mouse_passthrough(state: &mut OcrApp, mouse_passthrough: bool) {
    if mouse_passthrough {
        state.settings.quick_area_pick_mode = false;
    }
    state.settings.mouse_passthrough = mouse_passthrough;
}

fn update_decorations(ctx: &Context, state: &mut OcrApp, decorations: bool) {
    state.settings.decorations = decorations;
    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(decorations));
}

static EVENT_HANDLER: LazyLock<Arc<Mutex<Vec<Event>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Vec::default())));

pub fn emit_event(event: Event) {
    EVENT_HANDLER.lock().unwrap().push(event);
}

pub fn get_events() -> Vec<Event> {
    let mut handler = EVENT_HANDLER.lock().unwrap();
    std::mem::take(&mut handler)
}

#[cfg(test)]
mod tests {
    use crate::event::event::{Event, emit_event, get_events};

    #[test]
    fn emit_event_test() {
        emit_event(Event::ResetUi);
        emit_event(Event::ResetOcrStartTime);

        let result = get_events();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], Event::ResetUi);
        assert_eq!(result[1], Event::ResetOcrStartTime);

        let result = get_events();
        assert!(result.is_empty());
    }
}

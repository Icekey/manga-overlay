use crate::OcrApp;
use crate::action::ScreenshotResult;
use crate::database::{HistoryData, KanjiStatistic};
use crate::jpn::JpnData;
use crate::ui::image_display::ImageDisplayType;
use crate::ui::settings::{Backend, BackendStatus};
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{Context, Id, Memory};
use image::{DynamicImage, EncodableLayout};
use log::debug;
use std::cmp::max;
use std::ops::Add;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::time::Instant;

#[derive(PartialEq, Debug, Clone)]
pub enum Event {
    UpdateScreenshotResult(ScreenshotResult),
    UpdateHistoryData(Vec<HistoryData>),
    UpdateKanjiStatistic(Vec<KanjiStatistic>),
    UpdateSelectedJpnData(JpnData),
    UpdateBackendStatus(Backend, BackendStatus),
    ResetUi,
    ResetOcrStartTime,
    UpdateImageDisplay(ImageDisplayType, Option<DynamicImage>),
    RemovePipelineStep(usize),
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
            Event::UpdateImageDisplay(image_typ, image) => {
                update_image_display(ctx, state, image_typ, image)
            }
            Event::RemovePipelineStep(index) => remove_pipeline_step(state, index),
        }
    }
}

fn update_screenshot_result(ctx: &Context, state: &mut OcrApp, data: ScreenshotResult) {
    if ctx
        .data(|x| x.get_temp(Id::new("ocr_is_cancelled")))
        .unwrap_or(false)
    {
        return;
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
    *state = OcrApp::default();
    OcrApp::init_backends();
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
    image_type: ImageDisplayType,
    image: Option<DynamicImage>,
) {
    let index = if let ImageDisplayType::DEBUG(index) = image_type {
        index
    } else {
        0
    };

    let texture = create_texture(ctx, image.as_ref(), image_type.get_texture_name());

    let image_handles = &mut image_type
        .get_image_display(&mut state.settings)
        .image_handles;
    let vec_size = max(index + 1, image_handles.len());
    image_handles.resize(vec_size, None);

    image_handles[index] = texture;
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
    state.settings.pipeline_config.items.remove(index);
    state.settings.debug_image.image_handles.remove(index);
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

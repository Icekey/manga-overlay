use crate::action::ScreenshotResult;
use crate::database::{HistoryData, KanjiStatistic};
use crate::jpn::JpnData;
use crate::ui::settings::{Backend, BackendStatus};
use crate::OcrApp;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::ColorImage;
use egui::{Context, Id, Memory, TextureHandle};
use image::DynamicImage;
use std::sync::LazyLock;
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub enum Event {
    UpdateScreenshotResult(ScreenshotResult),
    UpdateHistoryData(Vec<HistoryData>),
    UpdateKanjiStatistic(Vec<KanjiStatistic>),
    UpdateSelectedJpnData(JpnData),
    UpdateBackendStatus(Backend, BackendStatus),
    ResetUi,
}

pub trait EventHandler {
    fn emit(&self, value: Event);

    fn get_events(&self) -> Vec<Event>;

    fn update_state(&self, state: &mut OcrApp) {
        let events = self.get_events();

        for x in events {
            self.handle_event(state, x);
        }
    }

    fn handle_event(&self, state: &mut OcrApp, event: Event);
}

static EVENT_LIST_ID: LazyLock<Id> = LazyLock::new(|| Id::new("EVENT_LIST"));

impl EventHandler for Context {
    fn emit(&self, value: Event) {
        self.data_mut(|x| {
            x.get_temp_mut_or_insert_with(*EVENT_LIST_ID, Vec::new)
                .push(value);
        });
    }

    fn get_events(&self) -> Vec<Event> {
        self.data_mut(|x| x.remove_temp(*EVENT_LIST_ID).unwrap_or_default())
    }

    fn handle_event(&self, state: &mut OcrApp, event: Event) {
        match event {
            Event::UpdateScreenshotResult(result) => {
                let background_rect = &mut state.background_rect;
                background_rect.screenshot_result = result;
                let settings = &state.settings;
                if settings.auto_restart_ocr {
                    background_rect.start_ocr_at = Some(Instant::now());
                }

                background_rect.capture_image_handle = create_texture(
                    self,
                    background_rect.screenshot_result.capture_image.as_ref(),
                    "capture_image_texture",
                );

                background_rect.debug_image_handle = create_texture(
                    self,
                    background_rect.screenshot_result.debug_image.as_ref(),
                    "debug_image_texture",
                );
            }
            Event::UpdateHistoryData(data) => {
                state.history.history_data = data;
            }
            Event::UpdateKanjiStatistic(data) => {
                state.kanji_statistic.kanji_statistic = data;
                if state.kanji_statistic.selected_kanji_index.is_none() {
                    state
                        .kanji_statistic
                        .update_selected_kanji_statistic(0, self);
                }
            }
            Event::UpdateSelectedJpnData(data) => {
                state.kanji_statistic.selected_jpn_data = data;
            }
            Event::UpdateBackendStatus(backend, status) => {
                backend.set_status(self, status);
            }
            Event::ResetUi => {
                self.memory_mut(|x| *x = Memory::default());
                *state = OcrApp::default();
                OcrApp::init_backends(self);
            }
        }
    }
}

fn create_texture(
    ctx: &Context,
    image: Option<&DynamicImage>,
    name: &str,
) -> Option<TextureHandle> {
    image.map(|image| {
        ctx.load_texture(
            name,
            ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.clone().as_bytes(),
            ),
            TextureOptions::default(),
        )
    })
}

use super::background_rect::BackgroundRect;
use super::kanji_history_ui::{init_history_updater, HistoryDataUi};
use super::kanji_statistic_ui::{init_kanji_statistic_updater, KanjiStatisticUi};
use super::settings::{AppSettings, Backend, BackendStatus};
use crate::detect::comictextdetector::DETECT_STATE;
use crate::ocr::manga_ocr::MANGA_OCR;
use crate::ui::event::Event::UpdateBackendStatus;
use crate::ui::event::EventHandler;
use crate::ui::shutdown::{shutdown_tasks, TASK_TRACKER};
use egui::Context;
use futures::join;
use std::sync::LazyLock;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct OcrApp {
    pub settings: AppSettings,
    pub background_rect: BackgroundRect,
    pub kanji_statistic: KanjiStatisticUi,
    pub history: HistoryDataUi,
}

impl OcrApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ocr_app: Self = if let Some(storage) = cc.storage {
            let storage: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            let ctx = &cc.egui_ctx;

            init_font(ctx);
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
                storage.settings.decorations,
            ));

            storage
        } else {
            Default::default()
        };

        init_history_updater(cc.egui_ctx.clone());
        init_kanji_statistic_updater(cc.egui_ctx.clone());

        Self::init_backends(&cc.egui_ctx);

        ocr_app
    }

    fn init_backends(ctx: &Context) {
        let ctx1 = ctx.clone();
        TASK_TRACKER.spawn(async move {
            let init1 = TASK_TRACKER.spawn(async { LazyLock::force(&MANGA_OCR) });
            let init2 = TASK_TRACKER.spawn(async { LazyLock::force(&DETECT_STATE) });
            let (result1, result2) = join!(init1, init2);

            ctx1.emit(UpdateBackendStatus(
                Backend::MangaOcr,
                if result1.is_ok() && result2.is_ok() {
                    BackendStatus::Ready
                } else {
                    BackendStatus::Error
                },
            ));
        });
    }

    fn show(&mut self, ctx: &Context) {
        if ctx.input(|i| i.viewport().close_requested()) {
            shutdown_tasks();
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        self.background_rect.show(ctx, &self.settings);

        self.settings.show(ctx);

        if self.settings.show_statistics {
            self.kanji_statistic.show(ctx);
        }
        if self.settings.show_history {
            self.history.show(ctx);
        }

        self.update_mouse_passthrough(ctx);

        if self.settings.show_debug_cursor {
            self.draw_mouse_position(ctx);
        }
    }
}

impl eframe::App for OcrApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.update_state(self);

        ctx.set_zoom_factor(self.settings.zoom_factor);

        self.show(ctx);
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.settings.clear_color.to_normalized_gamma_f32()
    }
}

fn init_font(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../resources/fonts/NotoSansJP-Regular.ttf"
        ))
        .into(),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

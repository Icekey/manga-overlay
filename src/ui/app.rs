use super::background_rect::BackgroundRect;
use super::ocr_rect::OcrUiRect;
use super::settings::AppSettings;
use super::show_ui::ShowUi;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct OcrApp {
    pub settings: AppSettings,
    pub background_rect: BackgroundRect,
}

impl OcrApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            let storage: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            let ctx = &cc.egui_ctx;
            // ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(
            //     storage.settings.mouse_passthrough,
            // ));

            init_font(ctx);
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
                storage.settings.decorations,
            ));

            return storage;
        }

        Default::default()
    }
}

fn init_font(ctx: &egui::Context) {
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

impl eframe::App for OcrApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.settings.clear_color.to_normalized_gamma_f32()
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(2.0);

        self.background_rect.show(ctx, &self.settings);

        self.settings.show(ctx);

        self.update_mouse_passthrough(ctx);

        self.draw_mouse_position(ctx);

        OcrUiRect::new("OcrRect".into())
            .with_rect(self.background_rect.get_rect())
            .show(ctx);
    }
}

use crate::ui::settings::AppSettings;
use eframe::epaint::TextureHandle;
use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, EnumIter)]
pub enum ImageDisplayType {
    CAPTURE,
    DEBUG,
    PREPROCESSED,
}

impl ImageDisplayType {
    pub const fn get_title(&self) -> &'static str {
        match self {
            ImageDisplayType::CAPTURE => "Capture Image",
            ImageDisplayType::DEBUG => "Debug Image",
            ImageDisplayType::PREPROCESSED => "Preprocessed Image",
        }
    }

    pub const fn get_texture_name(&self) -> &'static str {
        match self {
            ImageDisplayType::CAPTURE => "capture_image_texture",
            ImageDisplayType::DEBUG => "debug_image_texture",
            ImageDisplayType::PREPROCESSED => "preprocessed_image_texture",
        }
    }

    pub fn get_image_display<'a>(&'a self, settings: &'a mut AppSettings) -> &'a mut ImageDisplay {
        match self {
            ImageDisplayType::CAPTURE => &mut settings.capture_image,
            ImageDisplayType::DEBUG => &mut settings.debug_image,
            ImageDisplayType::PREPROCESSED => &mut settings.filtered_image,
        }
    }

    pub fn show_image_in_window(&self, ctx: &egui::Context, image_display: &ImageDisplay) {
        if !image_display.visible {
            return;
        }

        egui::Window::new(self.get_title()).show(ctx, |ui| {
            if let Some(texture) = &image_display.image_handle {
                ui.add(
                    egui::Image::new(texture)
                        .shrink_to_fit()
                        .corner_radius(10.0),
                );
            } else {
                ui.label("No Image");
            }
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct ImageDisplay {
    pub visible: bool,

    #[serde(skip)]
    pub image_handle: Option<TextureHandle>,
}

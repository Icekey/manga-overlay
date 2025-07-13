use crate::ui::settings::AppSettings;
use eframe::epaint::TextureHandle;
use egui::Slider;
use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, EnumIter)]
pub enum ImageDisplayType {
    CAPTURE,
    DEBUG(usize),
    PREPROCESSED,
}

impl ImageDisplayType {
    pub const fn get_title(&self) -> &'static str {
        match self {
            ImageDisplayType::CAPTURE => "Capture Image",
            ImageDisplayType::DEBUG(_) => "Debug Image",
            ImageDisplayType::PREPROCESSED => "Preprocessed Image",
        }
    }

    pub const fn get_texture_name(&self) -> &'static str {
        match self {
            ImageDisplayType::CAPTURE => "capture_image_texture",
            ImageDisplayType::DEBUG(_) => "debug_image_texture",
            ImageDisplayType::PREPROCESSED => "preprocessed_image_texture",
        }
    }

    pub fn get_image_display<'a>(&'a self, settings: &'a mut AppSettings) -> &'a mut ImageDisplay {
        match self {
            ImageDisplayType::CAPTURE => &mut settings.capture_image,
            ImageDisplayType::DEBUG(_) => &mut settings.debug_image,
            ImageDisplayType::PREPROCESSED => &mut settings.filtered_image,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct ImageDisplay {
    pub visible: bool,
    pub step: String,

    #[serde(skip)]
    pub image_handles: Vec<Option<TextureHandle>>,

    current_index: usize,
}

impl ImageDisplay {
    pub fn show_image_in_window(&mut self, ctx: &egui::Context, title: &str) {
        if !self.visible {
            return;
        }

        egui::Window::new(title).show(ctx, |ui| {
            let len = self.image_handles.len();
            if len > 1 {
                ui.horizontal(|ui| {
                    ui.add(Slider::new(&mut self.current_index, 0..=len - 1));
                    ui.label(self.step.to_string());
                });
            }

            if let Some(texture) = &self.image_handles.get(self.current_index).unwrap_or(&None) {
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

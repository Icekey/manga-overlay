use eframe::epaint::TextureHandle;
use egui::Slider;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct ImageDisplay {
    pub visible: bool,

    #[serde(skip)]
    pub image_handles: Vec<ImageWrapper>,

    current_index: usize,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct ImageWrapper {
    pub label: String,
    #[serde(skip)]
    pub image_handle: Option<TextureHandle>,
}

impl ImageDisplay {
    pub fn show_image_in_window(&mut self, ctx: &egui::Context, title: &str) {
        if !self.visible {
            return;
        }

        egui::Window::new(title).show(ctx, |ui| {
            let len = self.image_handles.len();
            if let Some(wrapper) = self.image_handles.get(self.current_index) {
                if len > 1 {
                    ui.horizontal(|ui| {
                        ui.add(Slider::new(&mut self.current_index, 0..=len - 1));
                        ui.label(&wrapper.label);
                    });
                }
                if let Some(texture) = &wrapper.image_handle {
                    ui.add(
                        egui::Image::new(texture)
                            .shrink_to_fit()
                            .corner_radius(10.0),
                    );
                } else {
                    ui.label("No Image");
                }
            }
        });
    }
}

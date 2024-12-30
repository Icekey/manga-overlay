use super::show_ui::ShowUi;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppSettings {
    pub clear_color: egui::Color32,
    pub mouse_passthrough: bool,
    pub decorations: bool,
}

impl ShowUi for AppSettings {
    fn show(&mut self, ctx: &egui::Context) {
        let window = egui::Window::new("Settings");
        window.show(ctx, |ui| {
            egui::widgets::global_theme_preference_buttons(ui);

            ui.horizontal(|ui| {
                ui.label("Background Color: ");
                ui.color_edit_button_srgba(&mut self.clear_color);
            });

            ui.checkbox(&mut self.mouse_passthrough, "Mouse Passthrough");

            if ui.checkbox(&mut self.decorations, "Decorations").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(self.decorations));
            }

            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

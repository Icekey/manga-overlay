pub trait ShowUi {
    fn show(&mut self, ctx: &egui::Context);
}

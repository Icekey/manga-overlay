use std::time::Duration;

use egui::{CentralPanel, TopBottomPanel};
use egui_extras::{Column, TableBuilder};
use tokio::time::sleep;

use crate::event::event::update_history_data;
use crate::ui::shutdown::TASK_TRACKER;
use crate::ui::update_queue::enqueue_update;
use crate::{action, database::HistoryData};

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct HistoryDataUi {
    pub history_data: Vec<HistoryData>,
}
pub fn init_history_updater() {
    TASK_TRACKER.spawn(async move {
        loop {
            let history_data = action::load_history();

            enqueue_update(|_, app| update_history_data(app, history_data));
            sleep(Duration::from_secs(1)).await;
        }
    });
}

impl HistoryDataUi {
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new("History").open(open).show(ctx, |ui| {
            TopBottomPanel::bottom("HistoryDataUi invisible bottom panel")
                .show_separator_line(false)
                .show_inside(ui, |_| ());
            CentralPanel::default().show_inside(ui, |ui| self.show_table(ui));
        });
    }

    fn show_table(&mut self, ui: &mut egui::Ui) {
        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::remainder())
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Timestamp");
                });
                header.col(|ui| {
                    ui.heading("OCR");
                });
                header.col(|ui| {
                    ui.heading("Translation");
                });
            })
            .body(|body| {
                body.rows(30.0, self.history_data.len(), |mut row| {
                    if let Some(value) = self.history_data.get(row.index()) {
                        row.col(|ui| {
                            ui.label(&value.created_at);
                        });
                        row.col(|ui| {
                            ui.label(&value.ocr);
                        });
                        row.col(|ui| {
                            if let Some(translation) = &value.translation {
                                ui.label(translation);
                            } else if ui.button("Translate").clicked() {
                                let ocr = value.ocr.clone();
                                TASK_TRACKER.spawn(async move {
                                    let _ = action::get_translation(&ocr).await;
                                });
                            }
                        });
                    }
                });
            });
    }
}

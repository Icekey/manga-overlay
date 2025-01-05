use std::time::Duration;

use egui::{CentralPanel, TopBottomPanel};
use egui_extras::{Column, TableBuilder};
use tokio::{spawn, time::sleep};

use crate::{action, database::HistoryData};

use super::channel_value::ChannelValue;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct HistoryDataUi {
    history_data: ChannelValue<Vec<HistoryData>>,
}
impl HistoryDataUi {
    pub fn init_updater(&self) {
        let tx = self.history_data.tx();

        spawn(async move {
            loop {
                let history_data = action::load_history().await;

                let _ = tx.send(history_data).await;
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.history_data.update();

        egui::Window::new("HistoryDataUi").show(ctx, |ui| {
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
                body.rows(30.0, self.history_data.value.len(), |mut row| {
                    if let Some(value) = self.history_data.value.get(row.index()) {
                        row.col(|ui| {
                            ui.label(&value.created_at);
                        });
                        row.col(|ui| {
                            ui.label(&value.ocr);
                        });
                        row.col(|ui| {
                            if let Some(translation) = &value.translation {
                                ui.label(translation);
                            } else {
                                if ui.button("Translate").clicked() {
                                    let ocr = value.ocr.clone();
                                    tokio::spawn(async move {
                                        let _ = action::get_translation(&ocr).await;
                                    });
                                }
                            }
                        });
                    }
                });
            });
    }
}

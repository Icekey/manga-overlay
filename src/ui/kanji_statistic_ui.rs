use std::time::Duration;

use egui::{CentralPanel, ScrollArea, Sense, SidePanel, TopBottomPanel};
use egui_extras::{Column, TableBuilder};
use tokio::{spawn, time::sleep};

use crate::{action, database::KanjiStatistic, jpn::JpnData};

use super::{channel_value::ChannelValue, screenshot_result_ui::show_jpn_data_info};

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct KanjiStatisticUi {
    kanji_statistic: ChannelValue<Vec<KanjiStatistic>>,
    selected_kanji_index: Option<usize>,
    selected_jpn_data: ChannelValue<JpnData>,
}

impl KanjiStatisticUi {
    pub fn init_updater(&self) {
        let tx = self.kanji_statistic.tx();

        spawn(async move {
            loop {
                let kanji_statistic = action::load_statistic().await;

                let _ = tx.send(kanji_statistic).await;
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if self.kanji_statistic.update() {
            if self.selected_kanji_index.is_none() {
                self.update_selected_kanji_statistic(0);
            }
        }

        self.selected_jpn_data.update();

        egui::Window::new("Kanji Statistic").show(ctx, |ui| {
            SidePanel::left("Kanji Statistic Side Panel").show_inside(ui, |ui| {
                self.show_table(ui);
            });
            TopBottomPanel::bottom("Kanji Statistic invisible bottom panel")
                .show_separator_line(false)
                .show_inside(ui, |_| ());
            CentralPanel::default().show_inside(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.set_width(600.0);
                    show_jpn_data_info(ui, &self.selected_jpn_data.value);
                });
            });
        });
    }

    fn show_table(&mut self, ui: &mut egui::Ui) {
        TableBuilder::new(ui)
            .sense(Sense::click())
            .column(Column::auto())
            .column(Column::auto())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Kanji");
                });
                header.col(|ui| {
                    ui.heading("Count");
                });
            })
            .body(|body| {
                body.rows(30.0, self.kanji_statistic.value.len(), |mut row| {
                    if let Some(value) = self.kanji_statistic.value.get(row.index()) {
                        row.set_selected(self.selected_kanji_index == Some(row.index()));

                        row.col(|ui| {
                            ui.label(&value.kanji);
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", &value.count));
                        });

                        if row.response().clicked() {
                            self.update_selected_kanji_statistic(row.index());
                        }
                    }
                });
            });
    }

    fn update_selected_kanji_statistic(&mut self, index: usize) {
        self.selected_kanji_index = Some(index);
        if let Some(kanji_statistic) = self.kanji_statistic.value.get(index) {
            let tx = self.selected_jpn_data.tx();
            let kanji = kanji_statistic.kanji.clone();
            spawn(async move {
                if let Some(jpn_data) = action::get_kanji_jpn_data(&kanji).await {
                    let _ = tx.send(jpn_data).await;
                };
            });
        }
    }
}

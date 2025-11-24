use std::time::Duration;

use egui::{CentralPanel, Context, ScrollArea, Sense, SidePanel, TopBottomPanel};
use egui_extras::{Column, TableBuilder};
use tokio::time::sleep;

use super::screenshot_result_ui::show_jpn_data_info;
use crate::event::event::{update_kanji_statistic, update_selected_jpn_data};
use crate::ui::shutdown::TASK_TRACKER;
use crate::ui::update_queue::enqueue_update;
use crate::{action, database::KanjiStatistic, jpn::JpnData};

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct KanjiStatisticUi {
    pub kanji_statistic: Vec<KanjiStatistic>,
    pub selected_kanji_index: Option<usize>,
    pub selected_jpn_data: JpnData,
}

pub fn init_kanji_statistic_updater() {
    TASK_TRACKER.spawn(async move {
        loop {
            let kanji_statistic = action::load_statistic();

            enqueue_update(|ctx, app| update_kanji_statistic(ctx, app, kanji_statistic));
            sleep(Duration::from_secs(1)).await;
        }
    });
}

impl KanjiStatisticUi {
    pub fn show(&mut self, ctx: &Context, open: &mut bool) {
        egui::Window::new("Kanji Statistic")
            .open(open)
            .show(ctx, |ui| {
                SidePanel::left("Kanji Statistic Side Panel").show_inside(ui, |ui| {
                    self.show_table(ui);
                });
                TopBottomPanel::bottom("Kanji Statistic invisible bottom panel")
                    .show_separator_line(false)
                    .show_inside(ui, |_| ());
                CentralPanel::default().show_inside(ui, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.set_width(600.0);
                        show_jpn_data_info(ui, &self.selected_jpn_data);
                    });
                });
            });
    }

    fn show_table(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
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
                body.rows(30.0, self.kanji_statistic.len(), |mut row| {
                    if let Some(value) = self.kanji_statistic.get(row.index()) {
                        row.set_selected(self.selected_kanji_index == Some(row.index()));

                        row.col(|ui| {
                            ui.label(&value.kanji);
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", &value.count));
                        });

                        if row.response().clicked() {
                            self.update_selected_kanji_statistic(row.index(), &ctx);
                        }
                    }
                });
            });
    }

    pub(crate) fn update_selected_kanji_statistic(&mut self, index: usize, _ctx: &Context) {
        self.selected_kanji_index = Some(index);
        if let Some(kanji_statistic) = self.kanji_statistic.get(index) {
            let kanji = kanji_statistic.kanji.clone();
            TASK_TRACKER.spawn(async move {
                if let Some(jpn_data) = action::get_kanji_jpn_data(&kanji).await {
                    enqueue_update(|_, app| update_selected_jpn_data(app, jpn_data));
                };
            });
        }
    }
}

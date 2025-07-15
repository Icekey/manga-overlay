mod history_data;
mod kanji_statistic;
mod table;

pub use history_data::HistoryData;
pub use history_data::load_full_history;
pub use history_data::load_history_data;
pub use history_data::store_ocr;
pub use history_data::store_ocr_translation;

pub use kanji_statistic::KanjiStatistic;
pub use kanji_statistic::increment_kanji_statistic;
pub use kanji_statistic::init_kanji_statistic;
pub use kanji_statistic::load_statistic;

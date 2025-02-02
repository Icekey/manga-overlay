use anyhow::{Ok, Result};
use rusqlite::{params, Connection, Row};
use serde::Serialize;

use super::table::create_table;

#[derive(Debug, Default, PartialEq, Serialize, serde::Deserialize, Clone)]
pub struct HistoryData {
    pub id: i32,
    pub created_at: String,
    pub updated_at: String,
    pub ocr: String,
    pub translation: Option<String>,
}

fn open_connection() -> Result<Connection> {
    create_table(
        "CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT current_timestamp,
            updated_at TEXT NOT NULL DEFAULT current_timestamp,
            ocr TEXT UNIQUE NOT NULL,
            translation TEXT
        )",
    )
}

pub fn store_ocr(ocr: &str) -> Result<()> {
    let conn = open_connection()?;

    conn.execute(
        "INSERT INTO history (ocr) VALUES (?1) \
            ON CONFLICT(ocr) DO NOTHING",
        params![ocr],
    )?;

    Ok(())
}

pub fn store_ocr_translation(ocr: &str, translation: &str) -> Result<()> {
    let conn = open_connection()?;

    conn.execute(
        "INSERT INTO history (ocr, translation) VALUES (?1, ?2) \
            ON CONFLICT(ocr) DO UPDATE SET translation = excluded.translation, updated_at = current_timestamp",
        params![ocr, translation],
    )?;

    Ok(())
}

impl HistoryData {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let id: i32 = row.get(0)?;
        let created_at: String = row.get(1)?;
        let updated_at: String = row.get(2)?;
        let ocr: String = row.get(3)?;
        let translation: Option<String> = row.get(4)?;

        rusqlite::Result::Ok(HistoryData {
            id,
            created_at,
            updated_at,
            ocr,
            translation,
        })
    }
}

pub fn load_history_data(ocr: &str) -> Result<HistoryData> {
    let conn = open_connection()?;

    let mut stmt = conn.prepare("SELECT * FROM history WHERE ocr = ?1")?;

    let history: HistoryData = stmt.query_row([ocr], HistoryData::from_row)?;

    Ok(history)
}

pub fn load_full_history() -> Result<Vec<HistoryData>> {
    let conn = open_connection()?;

    let mut stmt = conn.prepare("SELECT * FROM history ORDER BY updated_at DESC, id DESC")?;

    let history: Vec<HistoryData> = stmt
        .query_map([], HistoryData::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(history)
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::database::table::drop_table;

    use super::*;

    #[test]
    #[serial]
    fn store_and_load_history() {
        drop_table("history").unwrap();

        store_ocr("ocr1").unwrap();
        store_ocr("ocr2").unwrap();

        let vec = load_full_history().unwrap();

        assert_eq!(&vec[0].ocr, "ocr2");
        assert!(&vec[0].translation.is_none());
        assert_eq!(&vec[1].ocr, "ocr1");
        assert!(&vec[1].translation.is_none());

        std::thread::sleep(std::time::Duration::from_secs(2));

        store_ocr_translation("ocr1", "translation1").unwrap();

        let vec = dbg!(load_full_history().unwrap());

        assert_eq!(&vec[0].ocr, "ocr1");
        assert_eq!(&vec[0].translation, &Some("translation1".to_string()));
        assert_eq!(&vec[1].ocr, "ocr2");
        assert!(&vec[1].translation.is_none());
    }
}

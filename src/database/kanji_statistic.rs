use anyhow::{Ok, Result};
use rusqlite::{params, Connection, Row};
use serde::Serialize;

use super::table::create_table;

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Default)]
pub struct KanjiStatistic {
    pub id: i32,
    pub created_at: String,
    pub updated_at: String,
    pub kanji: String,
    pub count: i32,
}

fn open_connection() -> Result<Connection> {
    create_table(
        "CREATE TABLE IF NOT EXISTS statistic (
            id INTEGER PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT current_timestamp,
            updated_at TEXT NOT NULL DEFAULT current_timestamp,
            kanji TEXT UNIQUE NOT NULL,
            count INTEGER NOT NULL DEFAULT 0
        )",
    )
}

pub fn init_kanji_statistic(kanji: &str) -> Result<KanjiStatistic> {
    update_kanji_statistic(
        kanji,
        "INSERT INTO statistic (kanji) VALUES (?1) \
         ON CONFLICT(kanji) DO NOTHING",
    )
}

pub fn increment_kanji_statistic(kanji: &str) -> Result<KanjiStatistic> {
    update_kanji_statistic(
        kanji,
        "INSERT INTO statistic (kanji, count) VALUES (?1, 1) \
        ON CONFLICT(kanji) DO UPDATE SET count = count + 1, updated_at = current_timestamp",
    )
}

fn update_kanji_statistic(kanji: &str, query: &str) -> Result<KanjiStatistic> {
    let conn = open_connection()?;

    conn.execute(query, params![kanji])?;

    load_kanji_statistic(kanji)
}

pub fn load_kanji_statistic(kanji: &str) -> Result<KanjiStatistic> {
    let conn = open_connection()?;

    let mut stmt = conn.prepare("SELECT * FROM statistic WHERE kanji = ?1")?;

    let statistic: KanjiStatistic = stmt.query_row([kanji], KanjiStatistic::from_row)?;

    Ok(statistic)
}

impl KanjiStatistic {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let id: i32 = row.get(0)?;
        let created_at: String = row.get(1)?;
        let updated_at: String = row.get(2)?;
        let kanji: String = row.get(3)?;
        let count: i32 = row.get(4)?;

        rusqlite::Result::Ok(KanjiStatistic {
            id,
            created_at,
            updated_at,
            kanji,
            count,
        })
    }
}

pub fn load_statistic() -> Result<Vec<KanjiStatistic>> {
    let conn = open_connection()?;

    let mut stmt = conn.prepare("SELECT * FROM statistic ORDER BY count DESC")?;

    let statistics: Vec<KanjiStatistic> = stmt
        .query_map([], KanjiStatistic::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(statistics)
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::database::table::drop_table;

    use super::*;

    #[test]
    #[serial]
    fn store_and_load_statistic() {
        drop_table("statistic").unwrap();

        init_kanji_statistic("Test1").unwrap();
        increment_kanji_statistic("Test1").unwrap();

        increment_kanji_statistic("Test2").unwrap();
        increment_kanji_statistic("Test2").unwrap();

        let result = load_statistic().unwrap();

        assert_eq!(result.len(), 2);

        assert_eq!(result[0].kanji, "Test2");
        assert_eq!(result[0].count, 2);
        assert_eq!(result[1].kanji, "Test1");
        assert_eq!(result[1].count, 1);
    }

    #[test]
    #[serial]
    fn test_init_kanji_statistic() {
        drop_table("statistic").unwrap();

        const KANJI: &str = "kanji";
        init_kanji_statistic(KANJI).unwrap();

        let statistic = load_kanji_statistic(KANJI).unwrap();
        assert_eq!(statistic.count, 0);

        let statistic = load_kanji_statistic(KANJI).unwrap();
        assert_eq!(statistic.count, 0);
    }

    #[test]
    #[serial]
    fn test_increment_kanji_statistic() {
        drop_table("statistic").unwrap();

        const KANJI: &str = "kanji";

        let statistic = increment_kanji_statistic(KANJI).unwrap();
        assert_eq!(statistic.count, 1);

        let statistic = increment_kanji_statistic(KANJI).unwrap();
        assert_eq!(statistic.count, 2);
    }
}

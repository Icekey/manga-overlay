use std::{fs, path::PathBuf};

use anyhow::{Context, Ok, Result};
use rusqlite::Connection;

const DATABASE_FILENAME: &str = if cfg!(test) {
    "manga_overlay_test.db"
} else {
    "manga_overlay.db"
};

pub fn create_database() -> Result<Connection> {
    let output: PathBuf = get_output_path(DATABASE_FILENAME);

    Connection::open(&output).context("Could not create database")
}

pub fn create_table(create_table_query: &str) -> Result<Connection> {
    let conn = create_database()?;

    conn.execute(create_table_query, [])
        .context("could not create table")?;

    Ok(conn)
}

#[cfg(test)]
pub fn drop_table(table_name: &str) -> Result<()> {
    let conn = create_database()?;

    conn.execute(&format!("DROP TABLE IF EXISTS {table_name}"), [])?;

    Ok(())
}

fn get_output_path(filename: &str) -> PathBuf {
    let path_buf = std::env::current_dir()
        .expect("unable to get current_dir")
        .join("output");

    fs::create_dir_all(&path_buf)
        .unwrap_or_else(|_| panic!("Unable to create output directory: {:?}", &path_buf));

    path_buf.join(filename)
}

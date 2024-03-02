use std::path::PathBuf;

use chrono::TimeDelta;
use rand::random;
use rusqlite::Connection;
use sqids::Sqids;

use crate::models::{File, Url};

pub fn create_connection(data_directory: &PathBuf) -> Result<Connection, rusqlite::Error> {
    let db_path = data_directory.join("db.sqlite3");
    let conn = Connection::open(&db_path)?;
    File::register_table(&conn);
    Url::register_table(&conn);
    Ok(conn)
}

pub fn random_token() -> String {
    let data = random::<[u64; 1]>();
    let token = Sqids::builder()
        .min_length(10)
        .build()
        .unwrap()
        .encode(&data)
        .unwrap();
    // token[0..10].to_string()
    token
}

pub fn calculate_expires(
    size: usize,
    min_expires: usize,
    max_expires: usize,
    max_size: usize,
) -> TimeDelta {
    let min_expires = min_expires as i64;
    let max_expires = max_expires as i64;

    let millis = min_expires
        + ((-max_expires + min_expires) as f32 * (size as f32 / max_size as f32 - 1.0).powi(3))
            .floor() as i64;
    TimeDelta::milliseconds(millis)
}

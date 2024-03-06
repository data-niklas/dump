use std::path::PathBuf;

use crate::{mime::identify, util::random_token};
use chrono::{DateTime, TimeDelta, Utc};
use rusqlite::{Connection, OptionalExtension};
use sha256::digest;

pub struct DumpDetails {
    pub file_name: String,
    pub secret: Option<String>,
    pub expires: TimeDelta,
}

pub struct Dump {
    pub file_bytes: Vec<u8>,
    pub details: DumpDetails,
}

pub struct File {
    pub hash: String,
    pub size: usize,
    pub mime: String,
    pub group: String,
}

impl File {
    pub fn new(hash: String, size: usize, mime: String, group: String) -> File {
        File {
            hash,
            size,
            mime,
            group,
        }
    }

    pub fn from_dump(dump: &Dump, data_directory: &PathBuf) -> File {
        let hash = digest(&dump.file_bytes);
        let size = dump.file_bytes.len();

        let (_label, mime, group) = identify(data_directory, &dump.file_bytes);
        File {
            size,
            hash,
            mime,
            group,
        }
    }

    pub fn mime_count(conn: &Connection) -> Result<Vec<(String, usize)>, rusqlite::Error> {
        conn.prepare(
            "SELECT mime, COUNT(mime) AS count FROM files GROUP BY mime ORDER BY count DESC",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<(String, usize)>, rusqlite::Error>>()
    }

    pub fn group_count(conn: &Connection) -> Result<Vec<(String, usize)>, rusqlite::Error> {
        conn.prepare("SELECT file_type, COUNT(file_type) AS count FROM files GROUP BY file_type ORDER BY count DESC")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<(String, usize)>, rusqlite::Error>>()
    }

    pub fn count(conn: &Connection) -> Result<usize, rusqlite::Error> {
        conn.query_row("SELECT COUNT(1) FROM files", [], |row| Ok(row.get(0)?))
    }

    pub fn size_sum(conn: &Connection) -> Result<usize, rusqlite::Error> {
        conn.query_row("SELECT SUM(size) FROM files", [], |row| {
            let value: Option<u64> = row.get(0)?;
            Ok(value.unwrap_or(0) as usize)
        })
    }

    pub fn register_table(conn: &Connection) {
        let _ = conn
            .execute(
                "CREATE TABLE IF NOT EXISTS files (
          hash TEXT PRIMARY KEY,
          size INTEGER NOT NULL,
          mime TEXT,
          file_type TEXT
        )",
                (),
            )
            .unwrap();
    }
    pub fn search_file_by_hash(
        connection: &Connection,
        hash: &str,
    ) -> Result<Option<File>, rusqlite::Error> {
        connection
            .query_row("SELECT * FROM files WHERE hash = ?1", (hash,), |row| {
                Ok(File::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            })
            .optional()
    }

    pub fn create(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection
            .execute(
                "INSERT INTO files VALUES(?1, ?2, ?3, ?4)",
                (
                    self.hash.clone(),
                    self.size,
                    self.mime.clone(),
                    self.group.clone(),
                ),
            )
            .map(|_| ())
    }

    pub fn write(&self, data_dir: PathBuf, bytes: Vec<u8>) -> Result<(), std::io::Error> {
        let file_dir = data_dir.join("files");
        if !file_dir.exists() {
            std::fs::create_dir_all(&file_dir)?;
        }
        let file_path = file_dir.join(&self.hash);
        std::fs::write(file_path, &bytes)
    }

    pub fn read(&self, data_dir: PathBuf) -> Result<Vec<u8>, std::io::Error> {
        let file_path = data_dir.join("files").join(&self.hash);
        std::fs::read(file_path)
    }
    pub fn delete_unlinked(connection: &Connection) -> Result<(), rusqlite::Error> {
        connection
            .execute(
                "DELETE FROM files WHERE NOT EXISTS (SELECT 1 FROM urls WHERE files.hash = urls.file_hash)",
                (),
            )
            .map(|_| ())
    }

    pub fn search_unlinked(connection: &Connection) -> Result<Vec<File>, rusqlite::Error> {
        connection
            .prepare("SELECT * FROM files WHERE NOT EXISTS (SELECT 1 FROM urls WHERE files.hash = urls.file_hash)")?
            .query_map([], |row| {
                Ok(File::new(row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<Vec<File>, rusqlite::Error>>()
    }
}

pub struct Url {
    pub token: String,
    pub file_hash: String,
    pub secret: String,
    pub expires: DateTime<Utc>,
    pub file_name: String,
}

impl Url {
    pub fn new(
        token: String,
        file_hash: String,
        secret: String,
        expires: DateTime<Utc>,
        file_name: String,
    ) -> Self {
        Url {
            file_hash,
            token,
            secret,
            expires,
            file_name,
        }
    }

    pub fn count(conn: &Connection) -> Result<usize, rusqlite::Error> {
        conn.query_row("SELECT COUNT(1) FROM urls", [], |row| Ok(row.get(0)?))
    }

    pub fn from_dump_details_and_file(dump: &DumpDetails, file: &File) -> Url {
        let token = random_token();
        let secret = if dump.secret.is_some() {
            dump.secret.clone().unwrap()
        } else {
            random_token()
        };
        let expires = Utc::now() + dump.expires;
        Self::new(
            token,
            file.hash.clone(),
            secret,
            expires,
            dump.file_name.clone(),
        )
    }

    pub fn register_table(conn: &Connection) {
        let _ = conn
            .execute(
                "CREATE TABLE IF NOT EXISTS urls (
          token TEXT PRIMARY KEY,
          file_hash TEXT,
          secret TEXT,
          expires TEXT NOT NULL,
          file_name TEXT NOT NULL,
          FOREIGN KEY(file_hash) REFERENCES files(hash)
        )",
                (),
            )
            .unwrap();
    }
    pub fn create(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection
            .execute(
                "INSERT INTO urls VALUES(?1, ?2, ?3, ?4, ?5)",
                (
                    self.token.clone(),
                    self.file_hash.clone(),
                    self.secret.clone(),
                    self.expires.clone(),
                    self.file_name.clone(),
                ),
            )
            .map(|_| ())
    }

    pub fn delete(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection
            .execute("DELETE FROM urls WHERE token = ?1", (&self.token,))
            .map(|_| ())
    }

    pub fn search_url_by_token(
        connection: &Connection,
        token: &str,
    ) -> Result<Option<Url>, rusqlite::Error> {
        connection
            .query_row("SELECT * FROM urls WHERE token = ?1", (token,), |row| {
                Ok(Url::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .optional()
    }
    pub fn file(&self, connection: &Connection) -> Result<File, rusqlite::Error> {
        File::search_file_by_hash(&connection, &self.file_hash).map(Option::unwrap)
    }

    pub fn expired(&self) -> bool {
        self.expires < Utc::now()
    }

    pub fn delete_expired(connection: &Connection) -> Result<(), rusqlite::Error> {
        let expires = Utc::now();
        connection
            .execute("DELETE FROM urls WHERE expires < ?1", (expires,))
            .map(|_| ())
    }

    pub fn count_expired(connection: &Connection) -> Result<usize, rusqlite::Error> {
        let expires = Utc::now();
        connection.query_row(
            "SELECT COUNT(*) FROM urls WHERE expires < ?1",
            (expires,),
            |row| Ok(row.get(0)?),
        )
    }
}

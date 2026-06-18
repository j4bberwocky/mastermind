//! SQLite-backed persistent store for puzzles.
//!
//! Single shared `Arc<Mutex<Connection>>`: handlers take the lock, run the
//! query, release. Fine for LAN traffic; SQLite is single-writer anyway.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::{now_iso, Feedback, Problem, Settings};

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "io: {e}"),
            StorageError::Sqlite(e) => write!(f, "sqlite: {e}"),
            StorageError::Serde(e) => write!(f, "serde: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self { StorageError::Io(e) }
}
impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self { StorageError::Sqlite(e) }
}
impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self { StorageError::Serde(e) }
}

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Db {
    /// Open (or create) the database at `path`. Creates the parent directory if
    /// needed, enables WAL, and runs idempotent schema init.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        init_schema(&conn)?;
        Ok(Db {
            conn: Arc::new(Mutex::new(conn)),
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn insert(&self, p: &Problem) -> Result<(), StorageError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let guesses_json = serde_json::to_string(&p.initial_guesses)?;
        let feedback_json = serde_json::to_string(&p.initial_feedback)?;
        conn.execute(
            "INSERT INTO problems (
                id, code, code_length, num_colors, allow_duplicates, max_attempts,
                initial_guesses, initial_feedback, title, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                p.id,
                p.code,
                p.settings.code_length as i64,
                p.settings.num_colors as i64,
                p.settings.allow_duplicates as i64,
                p.settings.max_attempts as i64,
                guesses_json,
                feedback_json,
                p.title,
                p.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<Problem>, StorageError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let row = conn
            .query_row(
                "SELECT id, code, code_length, num_colors, allow_duplicates, max_attempts,
                        initial_guesses, initial_feedback, title, created_at
                 FROM problems WHERE id = ?1",
                params![id],
                row_to_problem,
            )
            .optional()?;
        row.transpose().map_err(Into::into)
    }

    pub fn list(&self) -> Result<Vec<Problem>, StorageError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, code, code_length, num_colors, allow_duplicates, max_attempts,
                    initial_guesses, initial_feedback, title, created_at
             FROM problems ORDER BY created_at DESC, id",
        )?;
        let rows = stmt.query_map([], row_to_problem)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r??);
        }
        Ok(out)
    }

    /// Returns `true` if a row was updated, `false` if the id was not found.
    pub fn update_title(&self, id: &str, title: Option<&str>) -> Result<bool, StorageError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let n = conn.execute(
            "UPDATE problems SET title = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, title, now_iso()],
        )?;
        Ok(n > 0)
    }

    /// Returns `true` if a row was deleted, `false` if the id was not found.
    pub fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let n = conn.execute("DELETE FROM problems WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }
}

fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS problems (
            id                  TEXT PRIMARY KEY,
            code                BLOB NOT NULL,
            code_length         INTEGER NOT NULL CHECK(code_length BETWEEN 2 AND 8),
            num_colors          INTEGER NOT NULL CHECK(num_colors BETWEEN 2 AND 8),
            allow_duplicates    INTEGER NOT NULL CHECK(allow_duplicates IN (0, 1)),
            max_attempts        INTEGER NOT NULL CHECK(max_attempts BETWEEN 1 AND 20),
            initial_guesses     TEXT NOT NULL DEFAULT '[]',
            initial_feedback    TEXT NOT NULL DEFAULT '[]',
            title               TEXT CHECK(title IS NULL OR length(title) <= 80),
            created_at          TEXT NOT NULL,
            updated_at          TEXT NOT NULL,
            CHECK(length(code) = code_length)
        );
        CREATE INDEX IF NOT EXISTS idx_problems_created_at
            ON problems(created_at DESC, id);",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn sample(id: &str, title: Option<&str>) -> Problem {
        Problem {
            id: id.to_string(),
            code: vec![0, 1, 2, 3],
            settings: Settings {
                code_length: 4,
                num_colors: 6,
                allow_duplicates: true,
                max_attempts: 10,
            },
            initial_guesses: vec![vec![5, 5, 5, 5]],
            initial_feedback: vec![Feedback { blacks: 0, whites: 0 }],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            title: title.map(str::to_string),
        }
    }

    fn open_mem() -> Db {
        Db::open(Path::new(":memory:")).expect("open :memory:")
    }

    #[test]
    fn insert_then_get_roundtrips_all_fields() {
        let db = open_mem();
        let p = sample("abc1234", Some("Domenica"));
        db.insert(&p).unwrap();
        let got = db.get("abc1234").unwrap().expect("found");
        assert_eq!(got.id, p.id);
        assert_eq!(got.code, p.code);
        assert_eq!(got.settings.code_length, 4);
        assert_eq!(got.settings.num_colors, 6);
        assert!(got.settings.allow_duplicates);
        assert_eq!(got.settings.max_attempts, 10);
        assert_eq!(got.initial_guesses, p.initial_guesses);
        assert_eq!(got.initial_feedback.len(), 1);
        assert_eq!(got.initial_feedback[0].blacks, 0);
        assert_eq!(got.initial_feedback[0].whites, 0);
        assert_eq!(got.title.as_deref(), Some("Domenica"));
        assert_eq!(got.created_at, p.created_at);
    }

    #[test]
    fn get_unknown_id_returns_none() {
        let db = open_mem();
        assert!(db.get("nope").unwrap().is_none());
    }

    #[test]
    fn list_orders_by_created_at_desc_then_id() {
        let db = open_mem();
        let mut a = sample("aaa1111", None);
        a.created_at = "2026-01-01T00:00:00Z".into();
        let mut b = sample("bbb2222", None);
        b.created_at = "2026-01-02T00:00:00Z".into();
        let mut c = sample("ccc3333", None);
        c.created_at = "2026-01-02T00:00:00Z".into(); // same as b
        db.insert(&a).unwrap();
        db.insert(&b).unwrap();
        db.insert(&c).unwrap();

        let list = db.list().unwrap();
        let ids: Vec<&str> = list.iter().map(|p| p.id.as_str()).collect();
        // 2026-01-02 first (b, c by id asc), then 2026-01-01 (a)
        assert_eq!(ids, vec!["bbb2222", "ccc3333", "aaa1111"]);
    }

    #[test]
    fn update_title_changes_title_and_returns_true() {
        let db = open_mem();
        db.insert(&sample("abc1234", Some("vecchio"))).unwrap();
        assert!(db.update_title("abc1234", Some("nuovo")).unwrap());
        let got = db.get("abc1234").unwrap().unwrap();
        assert_eq!(got.title.as_deref(), Some("nuovo"));
    }

    #[test]
    fn update_title_to_none_clears_title() {
        let db = open_mem();
        db.insert(&sample("abc1234", Some("titolo"))).unwrap();
        assert!(db.update_title("abc1234", None).unwrap());
        let got = db.get("abc1234").unwrap().unwrap();
        assert_eq!(got.title, None);
    }

    #[test]
    fn update_title_unknown_id_returns_false() {
        let db = open_mem();
        assert!(!db.update_title("nope", Some("x")).unwrap());
    }

    #[test]
    fn delete_removes_and_returns_true() {
        let db = open_mem();
        db.insert(&sample("abc1234", None)).unwrap();
        assert!(db.delete("abc1234").unwrap());
        assert!(db.get("abc1234").unwrap().is_none());
        assert!(db.list().unwrap().is_empty());
    }

    #[test]
    fn delete_unknown_id_returns_false() {
        let db = open_mem();
        assert!(!db.delete("nope").unwrap());
    }

    #[test]
    fn title_check_constraint_rejects_overlong_title() {
        let db = open_mem();
        let mut p = sample("abc1234", None);
        p.title = Some("x".repeat(81));
        // The DB CHECK should fail at insert time as defense in depth.
        let err = db.insert(&p).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("CHECK") || msg.contains("constraint"), "got: {msg}");
    }
}

/// Maps a result row to a `Result<Problem, StorageError>`. We can't return
/// `Problem` directly because JSON deserialisation can fail and rusqlite's
/// callback expects `rusqlite::Result`; so we return `Result<Problem, _>` and
/// unwrap at the call site.
fn row_to_problem(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Problem, StorageError>> {
    let id: String = row.get(0)?;
    let code: Vec<u8> = row.get(1)?;
    let code_length: i64 = row.get(2)?;
    let num_colors: i64 = row.get(3)?;
    let allow_duplicates: i64 = row.get(4)?;
    let max_attempts: i64 = row.get(5)?;
    let guesses_json: String = row.get(6)?;
    let feedback_json: String = row.get(7)?;
    let title: Option<String> = row.get(8)?;
    let created_at: String = row.get(9)?;

    let parse = || -> Result<Problem, StorageError> {
        let initial_guesses: Vec<Vec<u8>> = serde_json::from_str(&guesses_json)?;
        let initial_feedback: Vec<Feedback> = serde_json::from_str(&feedback_json)?;
        Ok(Problem {
            id,
            code,
            settings: Settings {
                code_length: code_length as usize,
                num_colors: num_colors as usize,
                allow_duplicates: allow_duplicates != 0,
                max_attempts: max_attempts as usize,
            },
            initial_guesses,
            initial_feedback,
            created_at,
            title,
        })
    };
    Ok(parse())
}

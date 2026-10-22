pub mod models;
pub mod queries;
pub mod schema;

pub use models::*;
pub use queries::*;

use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};

pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        let conn = Connection::open(&p)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        schema::init(&conn)?;
        Ok(Self { path: p })
    }

    pub fn conn(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(conn)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_open_and_conn() {
        let temp_dir = std::env::temp_dir().join("cine_test_db_pragma");
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("test.cinecat");
        if db_path.exists() {
            let _ = std::fs::remove_file(&db_path);
        }

        let db = Database::open(&db_path).expect("Failed to open database");
        let conn = db.conn().expect("Failed to get connection");

        // Verify journal mode is WAL
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("Failed to query journal_mode");
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

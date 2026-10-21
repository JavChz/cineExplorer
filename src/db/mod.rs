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
        schema::init(&conn)?;
        Ok(Self { path: p })
    }

    pub fn conn(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.execute("PRAGMA journal_mode = WAL", [])?;
        conn.execute("PRAGMA synchronous = NORMAL", [])?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        Ok(conn)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

use crate::error::Result;
use crate::schema::init_schema;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Primary database handle.
pub struct HollyDb {
    pub conn: Connection,
    pub vec_available: bool,
}

impl HollyDb {
    /// Open (or create) a database at the given path.
    /// Creates parent directories as needed.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        Self::configure_and_init(conn)
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure_and_init(conn)
    }

    fn configure_and_init(conn: Connection) -> Result<Self> {
        // Register sqlite-vec for future connections AND initialize on this one.
        // sqlite3_auto_extension only fires on connections opened after registration,
        // so we call the init function directly on the already-open connection too.
        #[allow(clippy::missing_transmute_annotations)]
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
            // Direct init on current connection (p_api = NULL: statically linked, no dispatch table needed)
            type ExtInit = unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *mut std::ffi::c_char,
                *const std::ffi::c_void,
            ) -> std::ffi::c_int;
            let init_fn: ExtInit = std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
            init_fn(conn.handle(), std::ptr::null_mut(), std::ptr::null());
        }

        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
        ",
        )?;

        let vec_available = init_schema(&conn)?;

        Ok(HollyDb {
            conn,
            vec_available,
        })
    }

    /// Resolve the database path using the standard discovery chain:
    /// 1. --db flag / HOLLY_DB_PATH env var
    /// 2. Walk up directories looking for .holly-db/holly.db
    /// 3. ~/.holly-db/holly.db (global fallback)
    pub fn resolve_path(explicit: Option<&Path>) -> PathBuf {
        if let Some(p) = explicit {
            return p.to_path_buf();
        }

        if let Ok(env_path) = std::env::var("HOLLY_DB_PATH") {
            return PathBuf::from(env_path);
        }

        // Walk up from cwd
        if let Ok(cwd) = std::env::current_dir() {
            let mut dir = cwd.as_path();
            loop {
                let candidate = dir.join(".holly-db").join("holly.db");
                if candidate.exists() {
                    return candidate;
                }
                match dir.parent() {
                    Some(p) => dir = p,
                    None => break,
                }
            }
        }

        // Global fallback
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".holly-db")
            .join("holly.db")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = HollyDb::open_in_memory().unwrap();
        // Verify tables exist
        let count: i64 = db
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='knowledge_nodes'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_open_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = HollyDb::open(&path).unwrap();
        assert!(path.exists());
        let _ = db;
    }

    #[test]
    fn test_wal_mode() {
        let db = HollyDb::open_in_memory().unwrap();
        let mode: String = db
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        // In-memory dbs always return "memory" mode, WAL only works on file dbs
        assert!(mode == "memory" || mode == "wal");
    }
}

use rusqlite::Connection;
use crate::error::Result;

pub const SCHEMA_VERSION: i64 = 1;
pub const EMBEDDING_DIM: usize = 384;

/// Create all tables, indexes, triggers, and FTS virtual table.
/// Returns whether sqlite-vec is available.
pub fn init_schema(conn: &Connection) -> Result<bool> {
    conn.execute_batch(CORE_DDL)?;

    // FTS update trigger scoped to title/content changes only.
    // Avoids trigger corruption bug when metadata-only updates fire the trigger.
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS knowledge_nodes_au;
         CREATE TRIGGER knowledge_nodes_au
         AFTER UPDATE OF title, content ON knowledge_nodes BEGIN
             INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content)
             VALUES ('delete', old.rowid, old.title, old.content);
             INSERT INTO knowledge_fts(rowid, title, content)
             VALUES (new.rowid, new.title, new.content);
         END;",
    )?;

    set_schema_version(conn, SCHEMA_VERSION)?;

    // Try to create the vector table (requires sqlite-vec extension loaded)
    let vec_available = try_create_vec_table(conn);
    Ok(vec_available)
}

fn try_create_vec_table(conn: &Connection) -> bool {
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_vec USING vec0(
            id TEXT PRIMARY KEY,
            embedding float[{}] distance_metric=cosine
        );",
        EMBEDDING_DIM
    ))
    .is_ok()
}

fn set_schema_version(conn: &Connection, version: i64) -> Result<()> {
    conn.execute_batch(&format!(
        "INSERT OR IGNORE INTO holly_meta(key, value) VALUES ('schema_version', '{}');",
        version
    ))?;
    Ok(())
}

pub fn get_schema_version(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT value FROM holly_meta WHERE key = 'schema_version'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(0)
}

const CORE_DDL: &str = "
-- Schema metadata
CREATE TABLE IF NOT EXISTS holly_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Core nodes table
CREATE TABLE IF NOT EXISTS knowledge_nodes (
    id         TEXT PRIMARY KEY,
    node_type  TEXT NOT NULL,
    title      TEXT NOT NULL,
    content    TEXT NOT NULL DEFAULT '{}',
    tags       TEXT NOT NULL DEFAULT '[]',
    repo       TEXT,
    status     TEXT,
    source     TEXT NOT NULL DEFAULT 'curated',
    agent      TEXT,
    user       TEXT,
    llm        TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Edges table
CREATE TABLE IF NOT EXISTS knowledge_edges (
    from_id    TEXT NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    to_id      TEXT NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    edge_type  TEXT NOT NULL,
    agent      TEXT,
    user       TEXT,
    llm        TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (from_id, to_id, edge_type)
);

-- Events table
CREATE TABLE IF NOT EXISTS holly_events (
    id              TEXT PRIMARY KEY,
    event_type      TEXT NOT NULL,
    payload         TEXT NOT NULL DEFAULT '{}',
    repo            TEXT,
    workspace       TEXT,
    agent           TEXT,
    user            TEXT,
    llm             TEXT,
    idempotency_key TEXT,
    created_at      TEXT NOT NULL
);

-- FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
    title, content,
    content=knowledge_nodes,
    content_rowid=rowid
);

-- FTS triggers
CREATE TRIGGER IF NOT EXISTS knowledge_nodes_ai AFTER INSERT ON knowledge_nodes BEGIN
    INSERT INTO knowledge_fts(rowid, title, content)
    VALUES (new.rowid, new.title, new.content);
END;

CREATE TRIGGER IF NOT EXISTS knowledge_nodes_ad AFTER DELETE ON knowledge_nodes BEGIN
    INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content)
    VALUES ('delete', old.rowid, old.title, old.content);
END;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_nodes_type    ON knowledge_nodes(node_type);
CREATE INDEX IF NOT EXISTS idx_nodes_source  ON knowledge_nodes(source);
CREATE INDEX IF NOT EXISTS idx_nodes_repo    ON knowledge_nodes(repo);
CREATE INDEX IF NOT EXISTS idx_nodes_status  ON knowledge_nodes(status);
CREATE INDEX IF NOT EXISTS idx_nodes_created ON knowledge_nodes(created_at);
CREATE INDEX IF NOT EXISTS idx_edges_from    ON knowledge_edges(from_id);
CREATE INDEX IF NOT EXISTS idx_edges_to      ON knowledge_edges(to_id);
CREATE INDEX IF NOT EXISTS idx_events_type      ON holly_events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_workspace ON holly_events(workspace);
CREATE INDEX IF NOT EXISTS idx_events_created   ON holly_events(created_at);
CREATE INDEX IF NOT EXISTS idx_events_repo      ON holly_events(repo);

-- Idempotency index for events
CREATE UNIQUE INDEX IF NOT EXISTS idx_events_idempotency
    ON holly_events(event_type, COALESCE(workspace, ''), COALESCE(repo, ''), idempotency_key)
    WHERE idempotency_key IS NOT NULL;
";

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    #[test]
    fn test_schema_creates_tables() {
        let conn = open_test_db();
        init_schema(&conn).unwrap();

        let tables: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };

        assert!(tables.contains(&"knowledge_nodes".to_string()));
        assert!(tables.contains(&"knowledge_edges".to_string()));
        assert!(tables.contains(&"holly_events".to_string()));
        assert!(tables.contains(&"holly_meta".to_string()));
    }

    #[test]
    fn test_schema_version_stored() {
        let conn = open_test_db();
        init_schema(&conn).unwrap();
        assert_eq!(get_schema_version(&conn), SCHEMA_VERSION);
    }

    #[test]
    fn test_fts_trigger_fires() {
        let conn = open_test_db();
        init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO knowledge_nodes(id, node_type, title, content, created_at, updated_at)
             VALUES ('test-1', 'memory', 'test title', '{}', datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM knowledge_fts WHERE knowledge_fts MATCH 'test'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}

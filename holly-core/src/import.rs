use crate::db::HollyDb;
use crate::error::{HollyError, Result};
use rusqlite::{params, Connection};
use serde_json::Value;
use uuid::Uuid;

/// Statistics from a migration run.
#[derive(Debug, Default, serde::Serialize)]
pub struct ImportStats {
    pub nodes_imported: usize,
    pub edges_imported: usize,
    pub events_imported: usize,
    pub nodes_skipped: usize,
    pub errors: Vec<String>,
}

impl HollyDb {
    /// Import from a legacy Holly SQLite database.
    ///
    /// Field mapping (from consumer contract):
    /// - `node_type`                    → `node_type`
    /// - `content.status`               → top-level `status`
    /// - `metadata.created_by_agent`    → `agent`
    /// - `metadata.created_by_llm`      → `llm`
    /// - `domain`                       → dropped
    /// - `from_node`/`to_node`          → `from_id`/`to_id`
    /// - Event `id` INTEGER             → UUID (new)
    /// - Event payload provenance       → top-level columns
    /// - All timestamps preserved exactly
    pub fn import_from(&self, legacy_path: &std::path::Path) -> Result<ImportStats> {
        let legacy = Connection::open(legacy_path)
            .map_err(|e| HollyError::Import(format!("Cannot open legacy DB: {}", e)))?;

        let mut stats = ImportStats::default();

        // 1. Import nodes
        let nodes = load_legacy_nodes(&legacy)
            .map_err(|e| HollyError::Import(format!("Failed to read nodes: {}", e)))?;

        for legacy_node in &nodes {
            match self.import_legacy_node(legacy_node) {
                Ok(_) => stats.nodes_imported += 1,
                Err(e) => {
                    stats.nodes_skipped += 1;
                    stats.errors.push(format!("Node {}: {}", legacy_node.id, e));
                }
            }
        }

        // 2. Import edges
        let edges = load_legacy_edges(&legacy)
            .map_err(|e| HollyError::Import(format!("Failed to read edges: {}", e)))?;

        for edge in &edges {
            match self.import_legacy_edge(edge) {
                Ok(_) => stats.edges_imported += 1,
                Err(e) => {
                    stats
                        .errors
                        .push(format!("Edge {}->{}: {}", edge.from_node, edge.to_node, e));
                }
            }
        }

        // 3. Import events
        let events = load_legacy_events(&legacy)
            .map_err(|e| HollyError::Import(format!("Failed to read events: {}", e)))?;

        for event in &events {
            match self.import_legacy_event(event) {
                Ok(_) => stats.events_imported += 1,
                Err(e) => {
                    stats.errors.push(format!(
                        "Event {}: {}",
                        event.id.as_deref().unwrap_or("?"),
                        e
                    ));
                }
            }
        }

        Ok(stats)
    }

    fn import_legacy_node(&self, n: &LegacyNode) -> Result<()> {
        // Extract status from content
        let status = n
            .content
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract provenance from metadata
        let agent = n
            .metadata
            .get("created_by_agent")
            .and_then(|v| v.as_str())
            .filter(|s| !s.starts_with("unknown"))
            .map(|s| s.to_string());
        let llm = n
            .metadata
            .get("created_by_llm")
            .and_then(|v| v.as_str())
            .filter(|s| !s.starts_with("unknown"))
            .map(|s| s.to_string());

        let tags_json = serde_json::to_string(&Vec::<String>::new())?;
        let content_json = serde_json::to_string(&n.content)?;

        // Apply status governance (normalize mode — don't fail on invalid)
        let normalized_status =
            crate::types::apply_status_governance(&n.node_type, status.as_deref(), false)?;

        // Insert directly to preserve timestamps
        self.conn.execute(
            "INSERT OR IGNORE INTO knowledge_nodes
             (id, node_type, title, content, tags, repo, status, source, agent, user, llm, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                n.id,
                n.node_type,
                n.title,
                content_json,
                tags_json,
                n.repo,
                normalized_status,
                n.source,
                agent,
                None::<String>,  // user — not in legacy
                llm,
                n.created_at,
                n.updated_at,
            ],
        )?;

        Ok(())
    }

    fn import_legacy_edge(&self, e: &LegacyEdge) -> Result<()> {
        let agent = e
            .properties
            .get("created_by_agent")
            .and_then(|v| v.as_str())
            .filter(|s| !s.starts_with("unknown"))
            .map(|s| s.to_string());
        let llm = e
            .properties
            .get("created_by_llm")
            .and_then(|v| v.as_str())
            .filter(|s| !s.starts_with("unknown"))
            .map(|s| s.to_string());

        self.conn.execute(
            "INSERT OR IGNORE INTO knowledge_edges
             (from_id, to_id, edge_type, agent, user, llm, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                e.from_node,
                e.to_node,
                e.edge_type,
                agent,
                None::<String>,
                llm,
                e.created_at,
            ],
        )?;
        Ok(())
    }

    fn import_legacy_event(&self, e: &LegacyEvent) -> Result<()> {
        let new_id = Uuid::new_v4().to_string();
        let agent = e
            .payload
            .get("created_by_agent")
            .and_then(|v| v.as_str())
            .filter(|s| !s.starts_with("unknown"))
            .map(|s| s.to_string());
        let llm = e
            .payload
            .get("created_by_llm")
            .and_then(|v| v.as_str())
            .filter(|s| !s.starts_with("unknown"))
            .map(|s| s.to_string());
        let idempotency_key = e
            .payload
            .get("idempotency_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let payload_json = serde_json::to_string(&e.payload)?;

        self.conn.execute(
            "INSERT OR IGNORE INTO holly_events
             (id, event_type, payload, repo, workspace, agent, user, llm, idempotency_key, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                new_id,
                e.event_type,
                payload_json,
                e.repo,
                e.workspace,
                agent,
                None::<String>,
                llm,
                idempotency_key,
                e.created_at,
            ],
        )?;
        Ok(())
    }
}

// Legacy data structs

struct LegacyNode {
    id: String,
    node_type: String,
    title: String,
    content: Value,
    source: String,
    repo: Option<String>,
    created_at: String,
    updated_at: String,
    metadata: Value,
}

struct LegacyEdge {
    from_node: String,
    to_node: String,
    edge_type: String,
    properties: Value,
    created_at: String,
}

struct LegacyEvent {
    id: Option<String>, // NULL in some intermediate schema rows; only used for error reporting
    event_type: String,
    workspace: Option<String>,
    repo: Option<String>,
    payload: Value,
    created_at: String,
}

fn load_legacy_nodes(conn: &Connection) -> rusqlite::Result<Vec<LegacyNode>> {
    let has_metadata = conn
        .prepare("PRAGMA table_info(knowledge_nodes)")?
        .query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?
        .any(|r| r.map(|n| n == "metadata").unwrap_or(false));

    let sql = if has_metadata {
        "SELECT id, node_type, title, content, source, repo, created_at, updated_at,
                COALESCE(metadata, '{}') as metadata
         FROM knowledge_nodes ORDER BY created_at"
    } else {
        "SELECT id, node_type, title, content, source, repo, created_at, updated_at,
                '{}' as metadata
         FROM knowledge_nodes ORDER BY created_at"
    };

    let mut stmt = conn.prepare(sql)?;
    let nodes = stmt
        .query_map([], |row| {
            let content_str: String = row.get(3)?;
            let metadata_str: String = row.get(8)?;
            Ok(LegacyNode {
                id: row.get(0)?,
                node_type: row.get(1)?,
                title: row.get(2)?,
                content: serde_json::from_str(&content_str).unwrap_or_default(),
                source: row.get(4)?,
                repo: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(nodes)
}

fn load_legacy_edges(conn: &Connection) -> rusqlite::Result<Vec<LegacyEdge>> {
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(knowledge_edges)")?
        .query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Handle both old (from_node/to_node) and new (from_id/to_id) column names
    let (from_col, to_col) = if cols.iter().any(|n| n == "from_node") {
        ("from_node", "to_node")
    } else {
        ("from_id", "to_id")
    };

    // properties column was removed in the intermediate Rust schema
    let props_expr = if cols.iter().any(|n| n == "properties") {
        "COALESCE(properties, '{}')".to_string()
    } else {
        "'{}'".to_string()
    };

    let sql = format!(
        "SELECT {from}, {to}, edge_type, {props}, created_at
         FROM knowledge_edges ORDER BY created_at",
        from = from_col,
        to = to_col,
        props = props_expr,
    );

    let mut stmt = conn.prepare(&sql)?;
    let edges = stmt
        .query_map([], |row| {
            let props_str: String = row.get(3)?;
            Ok(LegacyEdge {
                from_node: row.get(0)?,
                to_node: row.get(1)?,
                edge_type: row.get(2)?,
                properties: serde_json::from_str(&props_str).unwrap_or_default(),
                created_at: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::HollyDb;
    use rusqlite::Connection;

    /// Build a legacy DB with the original TypeScript schema (no metadata column).
    fn make_legacy_db_without_metadata() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE knowledge_nodes (
                id        TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                title     TEXT NOT NULL,
                content   TEXT NOT NULL DEFAULT '{}',
                tags      TEXT NOT NULL DEFAULT '[]',
                repo      TEXT,
                status    TEXT,
                source    TEXT NOT NULL DEFAULT 'curated',
                agent     TEXT,
                user      TEXT,
                llm       TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE knowledge_edges (
                from_id   TEXT NOT NULL,
                to_id     TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                properties TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE holly_events (
                id         INTEGER PRIMARY KEY,
                event_type TEXT NOT NULL,
                workspace  TEXT,
                repo       TEXT,
                payload    TEXT,
                created_at TEXT NOT NULL
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO knowledge_nodes (id, node_type, title, content, source, created_at, updated_at)
             VALUES ('test-id-1', 'decision', 'Test decision', '{\"detail\":\"value\"}',
                     'curated', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn
    }

    /// Build a DB with the intermediate Rust schema (has metadata column).
    fn make_legacy_db_with_metadata() -> Connection {
        let conn = make_legacy_db_without_metadata();
        conn.execute_batch("ALTER TABLE knowledge_nodes ADD COLUMN metadata TEXT;")
            .unwrap();
        conn.execute(
            "UPDATE knowledge_nodes SET metadata = '{\"created_by_agent\":\"claude-code\"}' WHERE id = 'test-id-1'",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_import_legacy_schema_without_metadata() {
        // Importing a DB that has no metadata column must not error.
        let legacy = make_legacy_db_without_metadata();
        let nodes = load_legacy_nodes(&legacy).expect("load_legacy_nodes should succeed");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, "test-id-1");
        assert_eq!(nodes[0].title, "Test decision");
        // metadata should default to empty object
        assert_eq!(nodes[0].metadata, serde_json::json!({}));
    }

    #[test]
    fn test_import_legacy_schema_with_metadata() {
        // Importing a DB that has a metadata column should read it.
        let legacy = make_legacy_db_with_metadata();
        let nodes = load_legacy_nodes(&legacy).expect("load_legacy_nodes should succeed");
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            nodes[0]
                .metadata
                .get("created_by_agent")
                .and_then(|v| v.as_str()),
            Some("claude-code")
        );
    }

    /// Write a legacy DB (no metadata column) to a temp file and return the path.
    fn write_legacy_db_to_tempfile() -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let conn = Connection::open(tmp.path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE knowledge_nodes (
                id        TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                title     TEXT NOT NULL,
                content   TEXT NOT NULL DEFAULT '{}',
                tags      TEXT NOT NULL DEFAULT '[]',
                repo      TEXT,
                status    TEXT,
                source    TEXT NOT NULL DEFAULT 'curated',
                agent     TEXT,
                user      TEXT,
                llm       TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE knowledge_edges (
                from_id   TEXT NOT NULL,
                to_id     TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                properties TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE holly_events (
                id         INTEGER PRIMARY KEY,
                event_type TEXT NOT NULL,
                workspace  TEXT,
                repo       TEXT,
                payload    TEXT,
                created_at TEXT NOT NULL
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO knowledge_nodes (id, node_type, title, content, source, created_at, updated_at)
             VALUES ('test-id-1', 'decision', 'Test decision', '{\"detail\":\"value\"}',
                     'curated', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        tmp
    }

    #[test]
    fn test_full_import_from_legacy_db_without_metadata() {
        // End-to-end: import_from succeeds on a schema without metadata column.
        let target = HollyDb::open_in_memory().unwrap();
        let tmp = write_legacy_db_to_tempfile();

        let stats = target
            .import_from(tmp.path())
            .expect("import_from should succeed");
        assert_eq!(stats.nodes_imported, 1);
        assert_eq!(stats.errors.len(), 0);

        let nodes = target.list_nodes(Default::default()).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].title, "Test decision");
    }
}

fn load_legacy_events(conn: &Connection) -> rusqlite::Result<Vec<LegacyEvent>> {
    // Check for repo column
    let has_repo = conn
        .prepare("PRAGMA table_info(holly_events)")?
        .query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?
        .any(|r| r.map(|n| n == "repo").unwrap_or(false));

    // Cast id to TEXT to handle both INTEGER (original TypeScript schema) and UUID TEXT
    let sql = if has_repo {
        "SELECT CAST(id AS TEXT), event_type, workspace, repo, payload, created_at
         FROM holly_events ORDER BY created_at"
    } else {
        "SELECT CAST(id AS TEXT), event_type, workspace, NULL, payload, created_at
         FROM holly_events ORDER BY created_at"
    };

    let mut stmt = conn.prepare(sql)?;
    let events = stmt
        .query_map([], |row| {
            let payload_str: String = row.get(4)?;
            Ok(LegacyEvent {
                id: row.get(0)?,
                event_type: row.get(1)?,
                workspace: row.get(2)?,
                repo: row.get(3)?,
                payload: serde_json::from_str(&payload_str).unwrap_or_default(),
                created_at: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(events)
}

use crate::db::HollyDb;
use crate::error::{HollyError, Result};
use crate::provenance::Provenance;
use chrono::Utc;
use rusqlite::params;

/// An edge between two nodes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    pub from_id: String,
    pub to_id: String,
    pub edge_type: String,
    pub agent: Option<String>,
    pub user: Option<String>,
    pub llm: Option<String>,
    pub created_at: String,
}

impl HollyDb {
    pub fn create_edge(
        &self,
        from_id: &str,
        to_id: &str,
        edge_type: &str,
        provenance: Option<Provenance>,
    ) -> Result<Edge> {
        // Validate both nodes exist
        self.get_node(from_id)?;
        self.get_node(to_id)?;

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let prov = provenance.unwrap_or_else(Provenance::from_env);

        self.conn.execute(
            "INSERT OR IGNORE INTO knowledge_edges
             (from_id, to_id, edge_type, agent, user, llm, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![from_id, to_id, edge_type, prov.agent, prov.user, prov.llm, now],
        )?;

        self.get_edge(from_id, to_id, edge_type)
    }

    pub fn get_edge(&self, from_id: &str, to_id: &str, edge_type: &str) -> Result<Edge> {
        self.conn
            .query_row(
                "SELECT from_id, to_id, edge_type, agent, user, llm, created_at
                 FROM knowledge_edges
                 WHERE from_id=?1 AND to_id=?2 AND edge_type=?3",
                params![from_id, to_id, edge_type],
                row_to_edge,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => HollyError::EdgeNotFound {
                    from: from_id.to_string(),
                    to: to_id.to_string(),
                    edge_type: edge_type.to_string(),
                },
                other => HollyError::Database(other),
            })
    }

    pub fn get_edges_from(&self, from_id: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_id, to_id, edge_type, agent, user, llm, created_at
             FROM knowledge_edges WHERE from_id=?1 ORDER BY created_at",
        )?;
        let edges = stmt
            .query_map(params![from_id], row_to_edge)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    pub fn get_edges_to(&self, to_id: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_id, to_id, edge_type, agent, user, llm, created_at
             FROM knowledge_edges WHERE to_id=?1 ORDER BY created_at",
        )?;
        let edges = stmt
            .query_map(params![to_id], row_to_edge)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    pub fn delete_edge(&self, from_id: &str, to_id: &str, edge_type: &str) -> Result<()> {
        let rows = self.conn.execute(
            "DELETE FROM knowledge_edges WHERE from_id=?1 AND to_id=?2 AND edge_type=?3",
            params![from_id, to_id, edge_type],
        )?;
        if rows == 0 {
            return Err(HollyError::EdgeNotFound {
                from: from_id.to_string(),
                to: to_id.to_string(),
                edge_type: edge_type.to_string(),
            });
        }
        Ok(())
    }

    /// Delete edges where either endpoint no longer exists.
    pub fn delete_orphaned_edges(&self) -> Result<usize> {
        let rows = self.conn.execute(
            "DELETE FROM knowledge_edges
             WHERE from_id NOT IN (SELECT id FROM knowledge_nodes)
                OR to_id   NOT IN (SELECT id FROM knowledge_nodes)",
            [],
        )?;
        Ok(rows)
    }
}

fn row_to_edge(row: &rusqlite::Row<'_>) -> rusqlite::Result<Edge> {
    Ok(Edge {
        from_id: row.get(0)?,
        to_id: row.get(1)?,
        edge_type: row.get(2)?,
        agent: row.get(3)?,
        user: row.get(4)?,
        llm: row.get(5)?,
        created_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::HollyDb;
    use crate::nodes::CreateNodeInput;

    fn test_db() -> HollyDb {
        HollyDb::open_in_memory().unwrap()
    }

    fn make_node(db: &HollyDb, title: &str) -> String {
        db.create_node(CreateNodeInput {
            node_type: "memory".into(),
            title: title.into(),
            ..Default::default()
        })
        .unwrap()
        .id
    }

    #[test]
    fn test_create_and_get_edge() {
        let db = test_db();
        let a = make_node(&db, "A");
        let b = make_node(&db, "B");

        let edge = db.create_edge(&a, &b, "relates_to", None).unwrap();
        assert_eq!(edge.from_id, a);
        assert_eq!(edge.to_id, b);
        assert_eq!(edge.edge_type, "relates_to");
    }

    #[test]
    fn test_edge_idempotent() {
        let db = test_db();
        let a = make_node(&db, "A");
        let b = make_node(&db, "B");

        db.create_edge(&a, &b, "relates_to", None).unwrap();
        // Second insert should not fail (INSERT OR IGNORE)
        db.create_edge(&a, &b, "relates_to", None).unwrap();

        let edges = db.get_edges_from(&a).unwrap();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_delete_edge() {
        let db = test_db();
        let a = make_node(&db, "A");
        let b = make_node(&db, "B");

        db.create_edge(&a, &b, "relates_to", None).unwrap();
        db.delete_edge(&a, &b, "relates_to").unwrap();

        let result = db.get_edge(&a, &b, "relates_to");
        assert!(matches!(result, Err(HollyError::EdgeNotFound { .. })));
    }

    #[test]
    fn test_cascade_delete() {
        let db = test_db();
        let a = make_node(&db, "A");
        let b = make_node(&db, "B");

        db.create_edge(&a, &b, "relates_to", None).unwrap();
        db.delete_node(&a).unwrap();

        // Edge should be gone due to CASCADE
        let result = db.get_edge(&a, &b, "relates_to");
        assert!(matches!(result, Err(HollyError::EdgeNotFound { .. })));
    }

    #[test]
    fn test_orphaned_edges_cleanup() {
        let db = test_db();
        let a = make_node(&db, "A");
        let b = make_node(&db, "B");
        db.create_edge(&a, &b, "relates_to", None).unwrap();

        // Disable FK enforcement to delete the node without cascade,
        // simulating an orphaned edge (e.g., from a legacy import or direct SQL)
        db.conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
        db.conn
            .execute("DELETE FROM knowledge_nodes WHERE id=?1", params![a])
            .unwrap();
        db.conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();

        let removed = db.delete_orphaned_edges().unwrap();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_edge_requires_valid_nodes() {
        let db = test_db();
        let result = db.create_edge("nonexistent", "also-nonexistent", "relates_to", None);
        assert!(result.is_err());
    }
}

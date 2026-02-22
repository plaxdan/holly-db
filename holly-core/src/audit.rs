use crate::db::HollyDb;
use crate::error::Result;
use rusqlite::params;

/// Audit results.
#[derive(Debug, serde::Serialize)]
pub struct AuditReport {
    pub stale_count: usize,
    pub stale_nodes: Vec<StaleNode>,
    pub orphaned_edges: usize,
    pub missing_embeddings: usize,
    pub empty_content_count: usize,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_events: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct StaleNode {
    pub id: String,
    pub node_type: String,
    pub title: String,
    pub status: Option<String>,
    pub updated_at: String,
    pub days_stale: i64,
}

impl HollyDb {
    /// Run a full audit of the database.
    pub fn audit(&self, stale_days: u32) -> Result<AuditReport> {
        let stale_nodes = self.find_stale_nodes(stale_days)?;
        let stale_count = stale_nodes.len();

        let orphaned_edges = self.count_orphaned_edges()?;
        let missing_embeddings = self.count_missing_embeddings()?;
        let empty_content_count = self.count_empty_content()?;
        let total_nodes = self.count_nodes()?;
        let total_edges = self.count_edges()?;
        let total_events = self.count_events()?;

        Ok(AuditReport {
            stale_count,
            stale_nodes,
            orphaned_edges,
            missing_embeddings,
            empty_content_count,
            total_nodes,
            total_edges,
            total_events,
        })
    }

    /// Count stale nodes (in-progress/active/open older than N days).
    pub fn count_stale_nodes(&self, stale_days: u32) -> Result<usize> {
        Ok(self.find_stale_nodes(stale_days)?.len())
    }

    fn find_stale_nodes(&self, stale_days: u32) -> Result<Vec<StaleNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, node_type, title, status, updated_at,
                    CAST((julianday('now') - julianday(updated_at)) AS INTEGER) as days_stale
             FROM knowledge_nodes
             WHERE status IN ('in_progress', 'active', 'open')
               AND julianday('now') - julianday(updated_at) > ?1
             ORDER BY updated_at ASC",
        )?;

        let nodes = stmt
            .query_map(params![stale_days as i64], |row| {
                Ok(StaleNode {
                    id: row.get(0)?,
                    node_type: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    updated_at: row.get(4)?,
                    days_stale: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    fn count_orphaned_edges(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM knowledge_edges
             WHERE from_id NOT IN (SELECT id FROM knowledge_nodes)
                OR to_id NOT IN (SELECT id FROM knowledge_nodes)",
            [],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }

    fn count_missing_embeddings(&self) -> Result<usize> {
        if !self.vec_available {
            return Ok(0);
        }
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM knowledge_nodes
             WHERE id NOT IN (SELECT id FROM knowledge_vec)",
            [],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }

    fn count_empty_content(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM knowledge_nodes WHERE content = '{}' OR content = '' OR content IS NULL",
            [],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }

    fn count_nodes(&self) -> Result<usize> {
        let c: i64 = self
            .conn
            .query_row("SELECT count(*) FROM knowledge_nodes", [], |r| r.get(0))?;
        Ok(c as usize)
    }

    fn count_edges(&self) -> Result<usize> {
        let c: i64 = self
            .conn
            .query_row("SELECT count(*) FROM knowledge_edges", [], |r| r.get(0))?;
        Ok(c as usize)
    }

    fn count_events(&self) -> Result<usize> {
        let c: i64 = self
            .conn
            .query_row("SELECT count(*) FROM holly_events", [], |r| r.get(0))?;
        Ok(c as usize)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::HollyDb;
    use crate::nodes::CreateNodeInput;

    #[test]
    fn test_audit_empty_db() {
        let db = HollyDb::open_in_memory().unwrap();
        let report = db.audit(14).unwrap();
        assert_eq!(report.stale_count, 0);
        assert_eq!(report.total_nodes, 0);
    }

    #[test]
    fn test_audit_counts() {
        let db = HollyDb::open_in_memory().unwrap();
        db.create_node(CreateNodeInput {
            node_type: "decision".into(),
            title: "Test decision".into(),
            ..Default::default()
        })
        .unwrap();

        let report = db.audit(14).unwrap();
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.stale_count, 0); // New node, not stale
    }
}

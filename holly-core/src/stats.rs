use crate::db::HollyDb;
use crate::error::Result;
use std::collections::HashMap;

/// Database statistics.
#[derive(Debug, serde::Serialize)]
pub struct Stats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_events: usize,
    pub by_type: HashMap<String, usize>,
    pub by_source: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
    pub daily_activity: Vec<DailyActivity>,
    pub edge_type_counts: HashMap<String, usize>,
}

#[derive(Debug, serde::Serialize)]
pub struct DailyActivity {
    pub date: String,
    pub count: usize,
}

impl HollyDb {
    pub fn stats(&self, days: u32) -> Result<Stats> {
        let total_nodes: usize = self
            .conn
            .query_row("SELECT count(*) FROM knowledge_nodes", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|c| c as usize)?;

        let total_edges: usize = self
            .conn
            .query_row("SELECT count(*) FROM knowledge_edges", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|c| c as usize)?;

        let total_events: usize = self
            .conn
            .query_row("SELECT count(*) FROM holly_events", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|c| c as usize)?;

        // By type
        let by_type = self.count_by("knowledge_nodes", "node_type")?;

        // By source
        let by_source = self.count_by("knowledge_nodes", "source")?;

        // By status
        let by_status = self.count_by_nullable("knowledge_nodes", "status")?;

        // Daily activity (parameterized; 0 = all time ~10 years)
        let window = if days == 0 { 3650 } else { days };
        let daily_activity = self.daily_activity(window)?;

        // Edge type counts
        let edge_type_counts = self.count_by("knowledge_edges", "edge_type")?;

        Ok(Stats {
            total_nodes,
            total_edges,
            total_events,
            by_type,
            by_source,
            by_status,
            daily_activity,
            edge_type_counts,
        })
    }

    fn count_by(&self, table: &str, column: &str) -> Result<HashMap<String, usize>> {
        let sql = format!(
            "SELECT {col}, count(*) FROM {tbl} GROUP BY {col}",
            col = column,
            tbl = table
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let map = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((key, count as usize))
            })?
            .collect::<std::result::Result<HashMap<_, _>, _>>()?;
        Ok(map)
    }

    fn count_by_nullable(&self, table: &str, column: &str) -> Result<HashMap<String, usize>> {
        let sql = format!(
            "SELECT COALESCE({col}, 'none'), count(*) FROM {tbl} GROUP BY {col}",
            col = column,
            tbl = table
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let map = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((key, count as usize))
            })?
            .collect::<std::result::Result<HashMap<_, _>, _>>()?;
        Ok(map)
    }

    fn daily_activity(&self, days: u32) -> Result<Vec<DailyActivity>> {
        let mut stmt = self.conn.prepare(
            "SELECT date(created_at) as day, count(*) as cnt
             FROM knowledge_nodes
             WHERE date(created_at) >= date('now', ?1)
             GROUP BY day
             ORDER BY day DESC",
        )?;
        let activity = stmt
            .query_map([format!("-{} days", days)], |row| {
                Ok(DailyActivity {
                    date: row.get(0)?,
                    count: row.get::<_, i64>(1)? as usize,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(activity)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::HollyDb;
    use crate::nodes::CreateNodeInput;

    #[test]
    fn test_stats_empty() {
        let db = HollyDb::open_in_memory().unwrap();
        let stats = db.stats(30).unwrap();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_edges, 0);
        assert_eq!(stats.total_events, 0);
    }

    #[test]
    fn test_stats_with_data() {
        let db = HollyDb::open_in_memory().unwrap();
        db.create_node(CreateNodeInput {
            node_type: "decision".into(),
            title: "test".into(),
            ..Default::default()
        })
        .unwrap();
        db.create_node(CreateNodeInput {
            node_type: "decision".into(),
            title: "test2".into(),
            ..Default::default()
        })
        .unwrap();
        db.create_node(CreateNodeInput {
            node_type: "constraint".into(),
            title: "c1".into(),
            ..Default::default()
        })
        .unwrap();

        let stats = db.stats(30).unwrap();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(*stats.by_type.get("decision").unwrap(), 2);
        assert_eq!(*stats.by_type.get("constraint").unwrap(), 1);
    }
}

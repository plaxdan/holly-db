use crate::db::HollyDb;
use crate::error::Result;
use crate::provenance::Provenance;
use chrono::Utc;
use rusqlite::params;
use serde_json::Value;
use uuid::Uuid;

/// A lifecycle event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HollyEvent {
    pub id: String,
    pub event_type: String,
    pub payload: Value,
    pub repo: Option<String>,
    pub workspace: Option<String>,
    pub agent: Option<String>,
    pub user: Option<String>,
    pub llm: Option<String>,
    pub idempotency_key: Option<String>,
    pub created_at: String,
}

/// Filters for listing events.
#[derive(Debug, Default)]
pub struct ListEventsFilter {
    pub event_type: Option<String>,
    pub repo: Option<String>,
    pub workspace: Option<String>,
    pub limit: Option<u32>,
}

impl HollyDb {
    /// Record an event. Returns the event, or the existing event if idempotency_key deduplicates it.
    pub fn record_event(
        &self,
        event_type: &str,
        payload: Value,
        repo: Option<&str>,
        workspace: Option<&str>,
        idempotency_key: Option<&str>,
        provenance: Option<Provenance>,
    ) -> Result<HollyEvent> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let prov = provenance.unwrap_or_else(Provenance::from_env);
        let payload_json = serde_json::to_string(&payload)?;

        self.conn.execute(
            "INSERT OR IGNORE INTO holly_events
             (id, event_type, payload, repo, workspace, agent, user, llm, idempotency_key, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                event_type,
                payload_json,
                repo,
                workspace,
                prov.agent,
                prov.user,
                prov.llm,
                idempotency_key,
                now,
            ],
        )?;

        // Return the stored event (may be the existing one if deduped)
        if let Some(key) = idempotency_key {
            if let Ok(existing) = self.conn.query_row(
                "SELECT id, event_type, payload, repo, workspace, agent, user, llm, idempotency_key, created_at
                 FROM holly_events
                 WHERE event_type=?1 AND COALESCE(workspace,'')=COALESCE(?2,'') AND COALESCE(repo,'')=COALESCE(?3,'') AND idempotency_key=?4",
                params![event_type, workspace, repo, key],
                row_to_event,
            ) {
                return Ok(existing);
            }
        }

        self.conn
            .query_row(
                "SELECT id, event_type, payload, repo, workspace, agent, user, llm, idempotency_key, created_at
                 FROM holly_events WHERE id=?1",
                params![id],
                row_to_event,
            )
            .map_err(Into::into)
    }

    pub fn list_events(&self, filter: ListEventsFilter) -> Result<Vec<HollyEvent>> {
        let mut sql = "SELECT id, event_type, payload, repo, workspace, agent, user, llm,
                              idempotency_key, created_at
                       FROM holly_events WHERE 1=1".to_string();
        let mut positional: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1usize;

        if let Some(ref t) = filter.event_type {
            sql.push_str(&format!(" AND event_type=?{idx}"));
            positional.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref r) = filter.repo {
            sql.push_str(&format!(" AND repo=?{idx}"));
            positional.push(Box::new(r.clone()));
            idx += 1;
        }
        if let Some(ref w) = filter.workspace {
            sql.push_str(&format!(" AND workspace=?{idx}"));
            positional.push(Box::new(w.clone()));
            idx += 1;
        }

        sql.push_str(" ORDER BY created_at DESC");

        let limit = filter.limit.unwrap_or(50);
        sql.push_str(&format!(" LIMIT ?{idx}"));
        positional.push(Box::new(limit));

        let refs: Vec<&dyn rusqlite::ToSql> = positional.iter().map(|b| b.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let events = stmt
            .query_map(refs.as_slice(), row_to_event)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(events)
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<HollyEvent> {
    let payload_str: String = row.get(2)?;
    let payload: Value = serde_json::from_str(&payload_str).unwrap_or(Value::Object(Default::default()));

    Ok(HollyEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        payload,
        repo: row.get(3)?,
        workspace: row.get(4)?,
        agent: row.get(5)?,
        user: row.get(6)?,
        llm: row.get(7)?,
        idempotency_key: row.get(8)?,
        created_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::HollyDb;

    fn test_db() -> HollyDb {
        HollyDb::open_in_memory().unwrap()
    }

    #[test]
    fn test_record_event() {
        let db = test_db();
        let event = db
            .record_event(
                "session_start",
                serde_json::json!({ "session": "abc" }),
                None,
                Some("my-workspace"),
                None,
                None,
            )
            .unwrap();

        assert_eq!(event.event_type, "session_start");
        assert_eq!(event.workspace.as_deref(), Some("my-workspace"));
    }

    #[test]
    fn test_idempotent_event() {
        let db = test_db();
        let e1 = db
            .record_event(
                "git_commit",
                serde_json::json!({}),
                Some("repo"),
                Some("ws"),
                Some("key-abc"),
                None,
            )
            .unwrap();

        let e2 = db
            .record_event(
                "git_commit",
                serde_json::json!({}),
                Some("repo"),
                Some("ws"),
                Some("key-abc"),
                None,
            )
            .unwrap();

        // Same event returned
        assert_eq!(e1.id, e2.id);

        // Only one event in DB
        let events = db.list_events(ListEventsFilter::default()).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_list_events_filter() {
        let db = test_db();
        db.record_event("session_start", serde_json::json!({}), None, None, None, None)
            .unwrap();
        db.record_event("git_commit", serde_json::json!({}), None, None, None, None)
            .unwrap();

        let commits = db
            .list_events(ListEventsFilter {
                event_type: Some("git_commit".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(commits.len(), 1);
    }
}

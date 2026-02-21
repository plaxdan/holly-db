use crate::db::HollyDb;
use crate::error::{HollyError, Result};
use crate::provenance::Provenance;
use crate::types::apply_status_governance;
use chrono::Utc;
use rusqlite::params;
use serde_json::Value;
use uuid::Uuid;

/// A knowledge node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: String,
    pub title: String,
    pub content: Value,
    pub tags: Vec<String>,
    pub repo: Option<String>,
    pub status: Option<String>,
    pub source: String,
    pub agent: Option<String>,
    pub user: Option<String>,
    pub llm: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a node.
#[derive(Debug, Default)]
pub struct CreateNodeInput {
    pub node_type: String,
    pub title: String,
    pub content: Option<Value>,
    pub tags: Vec<String>,
    pub repo: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub provenance: Option<Provenance>,
}

/// Filters for listing nodes.
#[derive(Debug, Default)]
pub struct ListNodesFilter {
    pub node_type: Option<String>,
    pub repo: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Input for updating a node.
#[derive(Debug, Default)]
pub struct UpdateNodeInput {
    pub title: Option<String>,
    pub content: Option<Value>,
    pub replace_content: bool,
    pub tags: Option<Vec<String>>,
    pub repo: Option<String>,
    pub status: Option<String>,
    pub provenance: Option<Provenance>,
}

impl HollyDb {
    pub fn create_node(&self, input: CreateNodeInput) -> Result<Node> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Merge content with type defaults
        let content = merge_content_with_defaults(&input.node_type, input.content);

        // Extract status from content if not provided explicitly
        let raw_status = input.status.as_deref().or_else(|| {
            content
                .get("status")
                .and_then(|v| v.as_str())
        });

        let status =
            apply_status_governance(&input.node_type, raw_status, false)?;

        let tags_json = serde_json::to_string(&input.tags)?;
        let content_json = serde_json::to_string(&content)?;
        let source = input.source.unwrap_or_else(|| "curated".into());
        let prov = input.provenance.unwrap_or_else(Provenance::from_env);

        self.conn.execute(
            "INSERT INTO knowledge_nodes
             (id, node_type, title, content, tags, repo, status, source, agent, user, llm, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                input.node_type,
                input.title,
                content_json,
                tags_json,
                input.repo,
                status,
                source,
                prov.agent,
                prov.user,
                prov.llm,
                now,
                now,
            ],
        )?;

        self.get_node(&id)
    }

    pub fn get_node(&self, id: &str) -> Result<Node> {
        self.conn
            .query_row(
                "SELECT id, node_type, title, content, tags, repo, status, source,
                        agent, user, llm, created_at, updated_at
                 FROM knowledge_nodes WHERE id = ?1",
                params![id],
                row_to_node,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    HollyError::NodeNotFound(id.to_string())
                }
                other => HollyError::Database(other),
            })
    }

    pub fn update_node(&self, id: &str, input: UpdateNodeInput) -> Result<Node> {
        let existing = self.get_node(id)?;
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let title = input.title.unwrap_or(existing.title.clone());

        let content = if let Some(new_content) = input.content {
            if input.replace_content {
                new_content
            } else {
                // Merge: new fields override existing fields
                let mut merged = existing.content.clone();
                if let (Some(obj), Some(new_obj)) =
                    (merged.as_object_mut(), new_content.as_object())
                {
                    for (k, v) in new_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
                merged
            }
        } else {
            existing.content.clone()
        };

        let raw_status = input.status.as_deref().or_else(|| {
            existing.status.as_deref()
        });
        let status = apply_status_governance(&existing.node_type, raw_status, false)?;

        let tags = input.tags.unwrap_or_else(|| existing.tags.clone());
        let repo = input.repo.or(existing.repo.clone());

        let tags_json = serde_json::to_string(&tags)?;
        let content_json = serde_json::to_string(&content)?;

        let prov = input.provenance.unwrap_or_else(Provenance::from_env);

        self.conn.execute(
            "UPDATE knowledge_nodes
             SET title=?2, content=?3, tags=?4, repo=?5, status=?6,
                 agent=?7, user=?8, llm=?9, updated_at=?10
             WHERE id=?1",
            params![
                id,
                title,
                content_json,
                tags_json,
                repo,
                status,
                prov.agent,
                prov.user,
                prov.llm,
                now,
            ],
        )?;

        self.get_node(id)
    }

    pub fn delete_node(&self, id: &str) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM knowledge_nodes WHERE id=?1", params![id])?;
        if rows == 0 {
            return Err(HollyError::NodeNotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn list_nodes(&self, filter: ListNodesFilter) -> Result<Vec<Node>> {
        let mut sql = "SELECT id, node_type, title, content, tags, repo, status, source,
                              agent, user, llm, created_at, updated_at
                       FROM knowledge_nodes WHERE 1=1".to_string();
        let mut positional: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1usize;

        if let Some(ref t) = filter.node_type {
            sql.push_str(&format!(" AND node_type=?{idx}"));
            positional.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref r) = filter.repo {
            sql.push_str(&format!(" AND repo=?{idx}"));
            positional.push(Box::new(r.clone()));
            idx += 1;
        }
        if let Some(ref s) = filter.status {
            sql.push_str(&format!(" AND status=?{idx}"));
            positional.push(Box::new(s.clone()));
            idx += 1;
        }
        if let Some(ref src) = filter.source {
            sql.push_str(&format!(" AND source=?{idx}"));
            positional.push(Box::new(src.clone()));
            idx += 1;
        }

        sql.push_str(" ORDER BY created_at DESC");

        let limit = filter.limit.unwrap_or(100);
        sql.push_str(&format!(" LIMIT ?{idx}"));
        positional.push(Box::new(limit));
        idx += 1;

        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET ?{idx}"));
            positional.push(Box::new(offset));
        }

        let refs: Vec<&dyn rusqlite::ToSql> = positional.iter().map(|b| b.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let nodes = stmt
            .query_map(refs.as_slice(), row_to_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(nodes)
    }
}

fn merge_content_with_defaults(node_type: &str, content: Option<Value>) -> Value {
    let defaults = default_content(node_type);
    let mut merged = defaults;
    if let Some(input) = content {
        if let (Some(obj), Some(input_obj)) = (merged.as_object_mut(), input.as_object()) {
            for (k, v) in input_obj {
                obj.insert(k.clone(), v.clone());
            }
        } else {
            merged = input;
        }
    }
    merged
}

fn default_content(node_type: &str) -> Value {
    match node_type {
        "idea" => serde_json::json!({ "source_channel": "cli", "raw_text": "", "status": "open" }),
        "goal" => serde_json::json!({ "priority": 5, "complexity": "medium", "status": "planning" }),
        "decision" => serde_json::json!({ "context": "", "decision": "", "consequences": "", "status": "proposed", "alternatives_considered": [] }),
        "implementation" => serde_json::json!({ "files": [], "commits": [], "status": "in_progress", "test_coverage": null }),
        "error" => serde_json::json!({ "stack_trace": "", "frequency": 1, "severity": "medium", "status": "open" }),
        "improvement" => serde_json::json!({ "rationale": "", "impact": "medium", "effort": "medium", "status": "proposed" }),
        "constraint" => serde_json::json!({ "applies_to": "", "value": "", "source_file": "", "verified_date": "", "status": "active" }),
        "task" => serde_json::json!({ "status": "planned", "priority": "medium", "owner": "", "depends_on": [], "evidence": [] }),
        "run" => serde_json::json!({ "status": "started", "task_id": "", "result": "recorded", "artifacts": [] }),
        "artifact" => serde_json::json!({ "status": "recorded", "artifact_type": "evidence", "path": "", "task_id": "", "run_id": "" }),
        _ => serde_json::json!({}),
    }
}

fn row_to_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<Node> {
    let content_str: String = row.get(3)?;
    let tags_str: String = row.get(4)?;

    let content: Value = serde_json::from_str(&content_str).unwrap_or(Value::Object(Default::default()));
    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

    Ok(Node {
        id: row.get(0)?,
        node_type: row.get(1)?,
        title: row.get(2)?,
        content,
        tags,
        repo: row.get(5)?,
        status: row.get(6)?,
        source: row.get(7)?,
        agent: row.get(8)?,
        user: row.get(9)?,
        llm: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
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
    fn test_create_and_get_node() {
        let db = test_db();
        let node = db
            .create_node(CreateNodeInput {
                node_type: "decision".into(),
                title: "Use SQLite".into(),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(node.node_type, "decision");
        assert_eq!(node.title, "Use SQLite");
        assert_eq!(node.status.as_deref(), Some("proposed"));
        assert_eq!(node.source, "curated");

        let fetched = db.get_node(&node.id).unwrap();
        assert_eq!(fetched.id, node.id);
    }

    #[test]
    fn test_update_node_merge() {
        let db = test_db();
        let node = db
            .create_node(CreateNodeInput {
                node_type: "decision".into(),
                title: "Original".into(),
                content: Some(serde_json::json!({ "context": "old context" })),
                ..Default::default()
            })
            .unwrap();

        let updated = db
            .update_node(
                &node.id,
                UpdateNodeInput {
                    title: Some("Updated".into()),
                    content: Some(serde_json::json!({ "decision": "use rust" })),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(updated.title, "Updated");
        // Merge: new field added, old field preserved
        assert_eq!(
            updated.content.get("decision").and_then(|v| v.as_str()),
            Some("use rust")
        );
        assert_eq!(
            updated.content.get("context").and_then(|v| v.as_str()),
            Some("old context")
        );
    }

    #[test]
    fn test_update_node_replace_content() {
        let db = test_db();
        let node = db
            .create_node(CreateNodeInput {
                node_type: "decision".into(),
                title: "Test".into(),
                content: Some(serde_json::json!({ "context": "old context" })),
                ..Default::default()
            })
            .unwrap();

        let updated = db
            .update_node(
                &node.id,
                UpdateNodeInput {
                    content: Some(serde_json::json!({ "decision": "new" })),
                    replace_content: true,
                    ..Default::default()
                },
            )
            .unwrap();

        // Replace: old field gone
        assert!(updated.content.get("context").is_none());
        assert_eq!(
            updated.content.get("decision").and_then(|v| v.as_str()),
            Some("new")
        );
    }

    #[test]
    fn test_delete_node() {
        let db = test_db();
        let node = db
            .create_node(CreateNodeInput {
                node_type: "memory".into(),
                title: "To delete".into(),
                ..Default::default()
            })
            .unwrap();

        db.delete_node(&node.id).unwrap();

        let result = db.get_node(&node.id);
        assert!(matches!(result, Err(HollyError::NodeNotFound(_))));
    }

    #[test]
    fn test_delete_nonexistent() {
        let db = test_db();
        let result = db.delete_node("nonexistent-id");
        assert!(matches!(result, Err(HollyError::NodeNotFound(_))));
    }

    #[test]
    fn test_list_nodes_with_filter() {
        let db = test_db();
        db.create_node(CreateNodeInput {
            node_type: "decision".into(),
            title: "Decision 1".into(),
            ..Default::default()
        })
        .unwrap();
        db.create_node(CreateNodeInput {
            node_type: "constraint".into(),
            title: "Constraint 1".into(),
            ..Default::default()
        })
        .unwrap();

        let decisions = db
            .list_nodes(ListNodesFilter {
                node_type: Some("decision".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].title, "Decision 1");
    }

    #[test]
    fn test_status_governance_on_create() {
        let db = test_db();
        // "in progress" should normalize to "in_progress" for task
        let node = db
            .create_node(CreateNodeInput {
                node_type: "task".into(),
                title: "Test task".into(),
                status: Some("in progress".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(node.status.as_deref(), Some("in_progress"));
    }

    #[test]
    fn test_status_governance_default() {
        let db = test_db();
        let node = db
            .create_node(CreateNodeInput {
                node_type: "task".into(),
                title: "Task no status".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(node.status.as_deref(), Some("planned"));
    }
}

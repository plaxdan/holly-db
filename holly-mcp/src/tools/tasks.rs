use holly_core::{
    CreateNodeInput, HollyDb, ListNodesFilter, Provenance, UpdateNodeInput,
};
use rmcp::model::{CallToolResult, Content};
use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};

type Db = Arc<Mutex<HollyDb>>;

fn ok(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text.into())])
}

fn err(text: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(text.into())])
}

fn get_str(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_str_req(args: &Map<String, Value>, key: &str) -> Result<String, String> {
    get_str(args, key).ok_or_else(|| format!("Missing required parameter: {}", key))
}

fn get_u32(args: &Map<String, Value>, key: &str) -> Option<u32> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

fn get_str_vec(args: &Map<String, Value>, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
}

/// holly_task_create — create a task node with task-specific fields.
pub async fn holly_task_create(db: Db, args: Map<String, Value>) -> CallToolResult {
    let title = match get_str_req(&args, "title") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let description = get_str(&args, "description").unwrap_or_default();
    let repo = get_str(&args, "repo");
    let source = get_str(&args, "source");
    let status = get_str(&args, "status").unwrap_or_else(|| "planned".to_string());
    let priority = get_str(&args, "priority").unwrap_or_else(|| "medium".to_string());
    let owner = get_str(&args, "owner").unwrap_or_default();
    let depends_on = get_str_vec(&args, "depends_on");
    let evidence = get_str_vec(&args, "evidence");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();

        let mut content = serde_json::json!({
            "status": status.clone(),
            "priority": priority,
            "owner": owner,
            "depends_on": depends_on,
            "evidence": evidence,
        });

        if !description.is_empty() {
            content["description"] = Value::String(description);
        }

        let input = CreateNodeInput {
            node_type: "task".to_string(),
            title: title.clone(),
            content: Some(content),
            repo,
            status: Some(status),
            source,
            provenance: Some(Provenance::from_env()),
            ..Default::default()
        };

        match db.create_node(input) {
            Ok(n) => ok(format!(
                "Created task \"{}\" ({}) status={}",
                n.title,
                n.id,
                n.status.as_deref().unwrap_or("planned")
            )),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_task_transition — transition a task's status.
pub async fn holly_task_transition(db: Db, args: Map<String, Value>) -> CallToolResult {
    let id = match get_str_req(&args, "id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let new_status = match get_str_req(&args, "status") {
        Ok(v) => v,
        Err(e) => return err(e),
    };

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();

        let existing = match db.get_node(&id) {
            Ok(n) => n,
            Err(e) => return err(format!("Error: {}", e)),
        };

        let old_status = existing.status.clone().unwrap_or_else(|| "unknown".to_string());

        let input = UpdateNodeInput {
            status: Some(new_status.clone()),
            provenance: Some(Provenance::from_env()),
            ..Default::default()
        };

        match db.update_node(&id, input) {
            Ok(_) => ok(format!("Task {} transitioned {} -> {}.", id, old_status, new_status)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_task_list — list task nodes with optional filters.
pub async fn holly_task_list(db: Db, args: Map<String, Value>) -> CallToolResult {
    let status = get_str(&args, "status");
    let repo = get_str(&args, "repo");
    let source = get_str(&args, "source");
    let limit = get_u32(&args, "limit").unwrap_or(20);

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        let filter = ListNodesFilter {
            node_type: Some("task".to_string()),
            status,
            repo,
            source,
            limit: Some(limit),
            ..Default::default()
        };
        match db.list_nodes(filter) {
            Ok(nodes) => {
                if nodes.is_empty() {
                    return ok("No tasks found.");
                }
                let lines: Vec<String> = nodes
                    .iter()
                    .map(|n| {
                        format!(
                            "[{}] {} ({}) status={} priority={}",
                            n.node_type,
                            n.title,
                            &n.id[..8.min(n.id.len())],
                            n.status.as_deref().unwrap_or("none"),
                            n.content.get("priority").and_then(|v| v.as_str()).unwrap_or("medium"),
                        )
                    })
                    .collect();
                ok(format!("Found {} task(s):\n\n{}", lines.len(), lines.join("\n")))
            }
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_run_start — create a run node linked to a task.
pub async fn holly_run_start(db: Db, args: Map<String, Value>) -> CallToolResult {
    let task_id = match get_str_req(&args, "task_id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let title = get_str(&args, "title").unwrap_or_else(|| format!("Run for {}", task_id));
    let repo = get_str(&args, "repo");
    let _workspace = get_str(&args, "workspace");
    let source = get_str(&args, "source");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();

        // Verify task exists
        if let Err(e) = db.get_node(&task_id) {
            return err(format!("Task not found: {}", e));
        }

        let content = serde_json::json!({
            "status": "started",
            "task_id": task_id.clone(),
            "result": "",
            "artifacts": [],
        });

        let run = db.create_node(CreateNodeInput {
            node_type: "run".to_string(),
            title: title.clone(),
            content: Some(content),
            repo,
            status: Some("started".to_string()),
            source,
            provenance: Some(Provenance::from_env()),
            ..Default::default()
        });

        match run {
            Ok(run_node) => {
                // Link run → task
                let _ = db.create_edge(&run_node.id, &task_id, "relates_to", None);

                // Try to transition task to in_progress if still planned
                if let Ok(task) = db.get_node(&task_id) {
                    if task.status.as_deref() == Some("planned") {
                        let _ = db.update_node(&task_id, UpdateNodeInput {
                            status: Some("in_progress".to_string()),
                            ..Default::default()
                        });
                    }
                }

                ok(format!(
                    "Started run \"{}\" ({}) for task {}.",
                    run_node.title, run_node.id, task_id
                ))
            }
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_run_complete — mark a run as complete.
pub async fn holly_run_complete(db: Db, args: Map<String, Value>) -> CallToolResult {
    let run_id = match get_str_req(&args, "run_id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let status = get_str(&args, "status").unwrap_or_else(|| "completed".to_string());
    let summary = get_str(&args, "summary").unwrap_or_default();
    let artifacts = get_str_vec(&args, "artifacts");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();

        // Get existing run to merge content
        // Verify run exists
        if let Err(e) = db.get_node(&run_id) {
            return err(format!("Run not found: {}", e));
        }

        let mut extra = Map::new();
        extra.insert("status".to_string(), Value::String(status.clone()));
        if !summary.is_empty() {
            extra.insert("result".to_string(), Value::String(summary));
        }
        if !artifacts.is_empty() {
            extra.insert("artifacts".to_string(), Value::Array(artifacts.into_iter().map(Value::String).collect()));
        }

        let input = UpdateNodeInput {
            content: Some(Value::Object(extra)),
            status: Some(status),
            provenance: Some(Provenance::from_env()),
            ..Default::default()
        };

        match db.update_node(&run_id, input) {
            Ok(n) => ok(format!("Completed run \"{}\" ({}).", n.title, n.id)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_task_link_artifact — create an artifact node linked to a task (and optionally run).
pub async fn holly_task_link_artifact(db: Db, args: Map<String, Value>) -> CallToolResult {
    let task_id = match get_str_req(&args, "task_id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let title = match get_str_req(&args, "title") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let path = match get_str_req(&args, "path") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let run_id = get_str(&args, "run_id");
    let artifact_type = get_str(&args, "artifact_type").unwrap_or_else(|| "evidence".to_string());
    let notes = get_str(&args, "notes").unwrap_or_default();
    let repo = get_str(&args, "repo");
    let source = get_str(&args, "source");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();

        // Verify task exists
        if let Err(e) = db.get_node(&task_id) {
            return err(format!("Task not found: {}", e));
        }

        let mut content = serde_json::json!({
            "status": "recorded",
            "artifact_type": artifact_type,
            "path": path,
            "task_id": task_id.clone(),
            "run_id": run_id.clone().unwrap_or_default(),
        });

        if !notes.is_empty() {
            content["notes"] = Value::String(notes);
        }

        let artifact = db.create_node(CreateNodeInput {
            node_type: "artifact".to_string(),
            title: title.clone(),
            content: Some(content),
            repo,
            status: Some("recorded".to_string()),
            source,
            provenance: Some(Provenance::from_env()),
            ..Default::default()
        });

        match artifact {
            Ok(a) => {
                // Link artifact → task
                let _ = db.create_edge(&a.id, &task_id, "relates_to", None);
                // Link artifact → run if provided
                if let Some(ref rid) = run_id {
                    if !rid.is_empty() {
                        let _ = db.create_edge(&a.id, rid, "relates_to", None);
                    }
                }

                ok(format!(
                    "Linked artifact \"{}\" ({}) to task {}.",
                    a.title, a.id, task_id
                ))
            }
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

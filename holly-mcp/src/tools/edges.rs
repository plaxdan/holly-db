use holly_core::HollyDb;
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

/// holly_connect — create an edge between two nodes.
pub async fn holly_connect(db: Db, args: Map<String, Value>) -> CallToolResult {
    let from_id = match get_str(&args, "from_id") {
        Some(v) => v,
        None => return err("Missing required parameter: from_id"),
    };
    let to_id = match get_str(&args, "to_id") {
        Some(v) => v,
        None => return err("Missing required parameter: to_id"),
    };
    let edge_type = get_str(&args, "edge_type").unwrap_or_else(|| "relates_to".to_string());

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.create_edge(&from_id, &to_id, &edge_type, None) {
            Ok(edge) => ok(format!(
                "Connected {} --{}--> {}",
                edge.from_id, edge.edge_type, edge.to_id
            )),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_delete_orphaned_edges — remove edges that reference deleted nodes.
pub async fn holly_delete_orphaned_edges(db: Db, _args: Map<String, Value>) -> CallToolResult {
    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.delete_orphaned_edges() {
            Ok(n) => ok(format!("Deleted {} orphaned edge(s).", n)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

use crate::content_parser::{extract_status, parse_content};
use crate::formatting::{format_node_detail, format_node_list, format_node_summary, format_recent_table};
use holly_core::{
    CreateNodeInput, HollyDb, ListNodesFilter, Node, Provenance, UpdateNodeInput,
    embeddings, embedding_text,
};
use rmcp::model::{CallToolResult, Content};
use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<HollyDb>>;

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

fn get_bool(args: &Map<String, Value>, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// holly_record — create a knowledge node from free-form content text.
pub async fn holly_record(db: Db, args: Map<String, Value>) -> CallToolResult {
    let node_type = match get_str_req(&args, "node_type") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let title = match get_str_req(&args, "title") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let content_text = get_str(&args, "content").unwrap_or_default();
    let repo = get_str(&args, "repo");
    let source = get_str(&args, "source");
    let _domain = get_str(&args, "domain");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();

        let parsed = if content_text.is_empty() {
            Value::Object(Map::new())
        } else {
            parse_content(&node_type, &content_text)
        };

        // Extract status from parsed content
        let status = extract_status(&content_text);

        let prov = Provenance::from_env();

        let node = db.create_node(CreateNodeInput {
            node_type: node_type.clone(),
            title: title.clone(),
            content: Some(parsed),
            tags: vec![],
            repo,
            status,
            source,
            provenance: Some(prov),
        });

        match node {
            Ok(n) => {
                // Generate and store embedding if model is available
                let model_dir = embeddings::default_model_dir();
                if db.vec_available && embeddings::model_available(&model_dir) {
                    let text = embedding_text(&n.title, &n.content);
                    if let Ok(emb) = embeddings::generate_embedding(&text) {
                        let _ = db.vec_upsert(&n.id, &emb);
                    }
                }
                ok(format!("Recorded [{}] \"{}\" ({})", n.node_type, n.title, n.id))
            }
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_get — fetch a node by ID with its edges.
pub async fn holly_get(db: Db, args: Map<String, Value>) -> CallToolResult {
    let id = match get_str_req(&args, "id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.get_node(&id) {
            Ok(node) => {
                let edges_from = db.get_edges_from(&id).unwrap_or_default();
                let edges_to = db.get_edges_to(&id).unwrap_or_default();
                ok(format_node_detail(&node, &edges_from, &edges_to))
            }
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_list — list nodes with optional filters.
pub async fn holly_list(db: Db, args: Map<String, Value>) -> CallToolResult {
    let node_type = get_str(&args, "node_type");
    let repo = get_str(&args, "repo");
    let status = get_str(&args, "status");
    let source = get_str(&args, "source");
    let limit = get_u32(&args, "limit");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        let filter = ListNodesFilter {
            node_type,
            repo,
            status,
            source,
            limit,
            ..Default::default()
        };
        match db.list_nodes(filter) {
            Ok(nodes) => ok(format_node_list(&nodes)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_recent — list recent nodes as a markdown table.
pub async fn holly_recent(db: Db, args: Map<String, Value>) -> CallToolResult {
    let days = get_u32(&args, "days").unwrap_or(7);
    let limit = get_u32(&args, "limit").unwrap_or(20);

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        let filter = ListNodesFilter {
            limit: Some(limit),
            since_days: Some(days),
            sort_by_updated: true,
            ..Default::default()
        };
        match db.list_nodes(filter) {
            Ok(nodes) => ok(format_recent_table(&nodes)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_update — update an existing node.
pub async fn holly_update(db: Db, args: Map<String, Value>) -> CallToolResult {
    let id = match get_str_req(&args, "id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let title = get_str(&args, "title");
    let content_text = get_str(&args, "content");
    let replace_content = get_bool(&args, "replace_content");
    let repo = get_str(&args, "repo");
    let new_status = get_str(&args, "status");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();

        let (parsed_content, extracted_status) = if let Some(ref text) = content_text {
            let node = match db.get_node(&id) {
                Ok(n) => n,
                Err(e) => return err(format!("Error: {}", e)),
            };
            let parsed = parse_content(&node.node_type, text);
            let status = extract_status(text);
            (Some(parsed), status)
        } else {
            (None, None)
        };

        let status = new_status.or(extracted_status);

        let input = UpdateNodeInput {
            title,
            content: parsed_content,
            replace_content,
            tags: None,
            repo,
            status,
            provenance: Some(Provenance::from_env()),
        };

        match db.update_node(&id, input) {
            Ok(n) => ok(format!("Updated [{}] \"{}\" ({})", n.node_type, n.title, n.id)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_delete — delete a node by ID.
pub async fn holly_delete(db: Db, args: Map<String, Value>) -> CallToolResult {
    let id = match get_str_req(&args, "id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.delete_node(&id) {
            Ok(()) => ok(format!("Deleted node {}.", id)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_related — find nodes semantically similar to a given node.
pub async fn holly_related(db: Db, args: Map<String, Value>) -> CallToolResult {
    let id = match get_str_req(&args, "id") {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let limit = get_u32(&args, "limit").unwrap_or(10);
    let node_type = get_str(&args, "node_type");

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.find_similar(&id, limit, node_type.as_deref()) {
            Ok(results) => {
                if results.is_empty() {
                    ok("No similar nodes found (vector search may not be available).")
                } else {
                    let nodes: Vec<&Node> = results.iter().map(|r| &r.node).collect();
                    let lines: Vec<String> = nodes.iter().map(|n| format_node_summary(n)).collect();
                    ok(format!("Found {} similar node(s):\n\n{}", lines.len(), lines.join("\n")))
                }
            }
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

use crate::formatting::{format_node_list, format_search_results};
use holly_core::{HollyDb, SearchOptions};
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

fn get_u32(args: &Map<String, Value>, key: &str) -> Option<u32> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

/// holly_search — semantic similarity search (hybrid FTS + vector).
pub async fn holly_search(db: Db, args: Map<String, Value>) -> CallToolResult {
    let query = match get_str(&args, "query") {
        Some(q) => q,
        None => return err("Missing required parameter: query"),
    };
    let node_type = get_str(&args, "node_type");
    let repo = get_str(&args, "repo");
    let source = get_str(&args, "source");
    let status = get_str(&args, "status");
    let limit = get_u32(&args, "limit").unwrap_or(20);

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        let opts = SearchOptions {
            node_type,
            repo,
            status,
            source,
            limit: Some(limit),
        };
        match db.hybrid_search(&query, None, opts) {
            Ok(results) => ok(format_search_results(&results)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_text_search — keyword/full-text search.
pub async fn holly_text_search(db: Db, args: Map<String, Value>) -> CallToolResult {
    let query = match get_str(&args, "query") {
        Some(q) => q,
        None => return err("Missing required parameter: query"),
    };
    let node_type = get_str(&args, "node_type");
    let repo = get_str(&args, "repo");
    let source = get_str(&args, "source");
    let status = get_str(&args, "status");
    let limit = get_u32(&args, "limit").unwrap_or(10);

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        let opts = SearchOptions {
            node_type,
            repo,
            status,
            source,
            limit: Some(limit),
        };
        match db.fts_search(&query, opts) {
            Ok(results) => {
                let nodes: Vec<_> = results.iter().map(|r| r.node.clone()).collect();
                ok(format_node_list(&nodes))
            }
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

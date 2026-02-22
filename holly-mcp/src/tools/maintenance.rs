use crate::formatting::{format_audit, format_stats};
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
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn get_u32(args: &Map<String, Value>, key: &str) -> Option<u32> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

/// holly_audit — audit the knowledge graph for health issues.
/// Note: similarity_threshold and duplicate_threshold are accepted but similarity/duplicate
/// detection is not yet implemented (returns 0 with a note).
pub async fn holly_audit(db: Db, args: Map<String, Value>) -> CallToolResult {
    let stale_days = get_u32(&args, "stale_days").unwrap_or(14);
    let mode = get_str(&args, "mode").unwrap_or_else(|| "summary".to_string());
    // Accept these params for API compatibility, but they're not yet implemented
    let _similarity_threshold = args
        .get("similarity_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.85);
    let _duplicate_threshold = args
        .get("duplicate_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.92);

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.audit(stale_days) {
            Ok(report) => ok(format_audit(&report, &mode)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_stats — show knowledge graph statistics.
pub async fn holly_stats(db: Db, args: Map<String, Value>) -> CallToolResult {
    let days = get_u32(&args, "days").unwrap_or(30);

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.stats(days) {
            Ok(stats) => ok(format_stats(&stats)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

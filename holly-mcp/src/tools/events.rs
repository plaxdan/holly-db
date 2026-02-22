use crate::formatting::format_event_list;
use holly_core::{HollyDb, ListEventsFilter};
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

/// holly_event_record — record a lifecycle event.
pub async fn holly_event_record(db: Db, args: Map<String, Value>) -> CallToolResult {
    let event_type = match get_str(&args, "event_type") {
        Some(v) => v,
        None => return err("Missing required parameter: event_type"),
    };
    let payload_str = get_str(&args, "payload").unwrap_or_else(|| "{}".to_string());
    let repo = get_str(&args, "repo");
    let workspace = get_str(&args, "workspace");

    let payload: Value = match serde_json::from_str(&payload_str) {
        Ok(v) => v,
        Err(e) => return err(format!("Invalid JSON payload: {}", e)),
    };

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        match db.record_event(
            &event_type,
            payload,
            repo.as_deref(),
            workspace.as_deref(),
            None,
            None,
        ) {
            Ok(event) => ok(format!(
                "Recorded event #{}: {}",
                &event.id[..8.min(event.id.len())],
                event.event_type
            )),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

/// holly_event_list — list lifecycle events with optional filters.
pub async fn holly_event_list(db: Db, args: Map<String, Value>) -> CallToolResult {
    let event_type = get_str(&args, "event_type");
    let repo = get_str(&args, "repo");
    let workspace = get_str(&args, "workspace");
    let limit = get_u32(&args, "limit").unwrap_or(50);

    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        let filter = ListEventsFilter {
            event_type,
            repo,
            workspace,
            limit: Some(limit),
        };
        match db.list_events(filter) {
            Ok(events) => ok(format_event_list(&events)),
            Err(e) => err(format!("Error: {}", e)),
        }
    })
    .await
    .unwrap_or_else(|e| err(format!("Internal error: {}", e)))
}

use holly_core::{HollyDb, ListEventsFilter};
use serde_json::Value;

pub fn record(
    db: &HollyDb,
    event_type: &str,
    payload: Option<&str>,
    repo: Option<&str>,
    workspace: Option<&str>,
    idempotency_key: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let payload_value: Value = payload
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid JSON payload: {}", e))?
        .unwrap_or(serde_json::json!({}));

    let event = db.record_event(
        event_type,
        payload_value,
        repo,
        workspace,
        idempotency_key,
        None,
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&event)?);
    } else {
        println!("Event recorded: {} [{}]", event.event_type, &event.id[..8]);
    }
    Ok(())
}

pub fn list(
    db: &HollyDb,
    event_type: Option<&str>,
    repo: Option<&str>,
    workspace: Option<&str>,
    limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let events = db.list_events(ListEventsFilter {
        event_type: event_type.map(|s| s.to_string()),
        repo: repo.map(|s| s.to_string()),
        workspace: workspace.map(|s| s.to_string()),
        limit: Some(limit),
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&events)?);
        return Ok(());
    }

    if events.is_empty() {
        println!("No events found.");
        return Ok(());
    }

    println!("{} event(s):\n", events.len());
    for e in &events {
        let ws = e
            .workspace
            .as_deref()
            .map(|w| format!(" ws:{}", w))
            .unwrap_or_default();
        println!("  {} [{}]{} {}", e.event_type, &e.id[..8], ws, e.created_at);
    }
    Ok(())
}

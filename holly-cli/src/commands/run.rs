use holly_core::{CreateNodeInput, HollyDb, UpdateNodeInput};

pub fn start(db: &HollyDb, task_id: &str, title: Option<&str>, json: bool) -> anyhow::Result<()> {
    // Verify task exists
    let task = db.get_node(task_id)?;

    let run_title = title
        .map(|t| t.to_string())
        .unwrap_or_else(|| format!("Run for: {}", task.title));

    let content = serde_json::json!({
        "status": "started",
        "task_id": task_id,
        "result": "recorded",
        "artifacts": []
    });

    let node = db.create_node(CreateNodeInput {
        node_type: "run".into(),
        title: run_title,
        content: Some(content),
        status: Some("started".into()),
        ..Default::default()
    })?;

    // Link run to task
    let _ = db.create_edge(&node.id, task_id, "derives_from", None);

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Run started [{}]: {}", &node.id[..8], node.title);
    }
    Ok(())
}

pub fn complete(
    db: &HollyDb,
    run_id: &str,
    status: Option<&str>,
    summary: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let final_status = status.unwrap_or("completed");
    let mut updates = serde_json::json!({});
    if let Some(s) = summary {
        updates["summary"] = serde_json::Value::String(s.to_string());
    }

    let node = db.update_node(
        run_id,
        UpdateNodeInput {
            status: Some(final_status.into()),
            content: if updates != serde_json::json!({}) {
                Some(updates)
            } else {
                None
            },
            ..Default::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Run {} [{}]: {}", final_status, &node.id[..8], node.title);
    }
    Ok(())
}

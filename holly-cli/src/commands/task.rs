use holly_core::{HollyDb, CreateNodeInput, UpdateNodeInput, ListNodesFilter};

pub fn create(
    db: &HollyDb,
    title: &str,
    repo: Option<&str>,
    priority: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let content = serde_json::json!({
        "status": "planned",
        "priority": priority.unwrap_or("medium"),
        "owner": "",
        "depends_on": [],
        "evidence": []
    });

    let node = db.create_node(CreateNodeInput {
        node_type: "task".into(),
        title: title.to_string(),
        content: Some(content),
        repo: repo.map(|s| s.to_string()),
        status: Some("planned".into()),
        ..Default::default()
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Task created [{}]: {}", &node.id[..8], node.title);
    }
    Ok(())
}

pub fn start(db: &HollyDb, id: &str, json: bool) -> anyhow::Result<()> {
    let node = db.update_node(
        id,
        UpdateNodeInput {
            status: Some("in_progress".into()),
            ..Default::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Task started [{}]: {}", &node.id[..8], node.title);
    }
    Ok(())
}

pub fn complete(db: &HollyDb, id: &str, json: bool) -> anyhow::Result<()> {
    let node = db.update_node(
        id,
        UpdateNodeInput {
            status: Some("completed".into()),
            ..Default::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Task completed [{}]: {}", &node.id[..8], node.title);
    }
    Ok(())
}

pub fn block(db: &HollyDb, id: &str, json: bool) -> anyhow::Result<()> {
    let node = db.update_node(
        id,
        UpdateNodeInput {
            status: Some("blocked".into()),
            ..Default::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Task blocked [{}]: {}", &node.id[..8], node.title);
    }
    Ok(())
}

pub fn cancel(db: &HollyDb, id: &str, json: bool) -> anyhow::Result<()> {
    let node = db.update_node(
        id,
        UpdateNodeInput {
            status: Some("cancelled".into()),
            ..Default::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Task cancelled [{}]: {}", &node.id[..8], node.title);
    }
    Ok(())
}

pub fn list(db: &HollyDb, status: Option<&str>, json: bool) -> anyhow::Result<()> {
    let nodes = db.list_nodes(ListNodesFilter {
        node_type: Some("task".into()),
        status: status.map(|s| s.to_string()),
        limit: Some(50),
        ..Default::default()
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&nodes)?);
        return Ok(());
    }

    if nodes.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    for n in &nodes {
        let s = n.status.as_deref().unwrap_or("?");
        println!("  [{}] {} — {}", &n.id[..8], s, n.title);
    }
    Ok(())
}

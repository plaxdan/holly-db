use holly_core::HollyDb;

pub fn run(db: &HollyDb, id: &str, related: bool, json: bool) -> anyhow::Result<()> {
    let node = db.get_node(id)?;

    if json {
        let mut output = serde_json::to_value(&node)?;
        if related {
            let edges_from = db.get_edges_from(id)?;
            let edges_to = db.get_edges_to(id)?;
            if let Some(obj) = output.as_object_mut() {
                obj.insert("edges_from".into(), serde_json::to_value(edges_from)?);
                obj.insert("edges_to".into(), serde_json::to_value(edges_to)?);
            }
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("ID:      {}", node.id);
    println!("Type:    {}", node.node_type);
    println!("Title:   {}", node.title);
    if let Some(s) = &node.status {
        println!("Status:  {}", s);
    }
    if let Some(r) = &node.repo {
        println!("Repo:    {}", r);
    }
    println!("Source:  {}", node.source);
    println!("Created: {}", node.created_at);
    println!("Updated: {}", node.updated_at);
    if node.content != serde_json::json!({}) {
        println!("Content:\n{}", serde_json::to_string_pretty(&node.content)?);
    }

    if related {
        let edges_from = db.get_edges_from(id)?;
        let edges_to = db.get_edges_to(id)?;
        if !edges_from.is_empty() {
            println!("\nLinks from this node:");
            for e in &edges_from {
                println!("  --[{}]--> {}", e.edge_type, e.to_id);
            }
        }
        if !edges_to.is_empty() {
            println!("\nLinks to this node:");
            for e in &edges_to {
                println!("  {} --[{}]-->", e.from_id, e.edge_type);
            }
        }
    }
    Ok(())
}

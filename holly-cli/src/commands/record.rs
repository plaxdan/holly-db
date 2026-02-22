use holly_core::{HollyDb, CreateNodeInput};
use serde_json::Value;

#[allow(clippy::too_many_arguments)]
pub fn run(
    db: &HollyDb,
    node_type: &str,
    title: &str,
    content: Option<&str>,
    repo: Option<&str>,
    status: Option<&str>,
    source: Option<&str>,
    tags: Vec<String>,
    json: bool,
) -> anyhow::Result<()> {
    let content_value: Option<Value> = content
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid JSON content: {}", e))?;

    let node = db.create_node(CreateNodeInput {
        node_type: node_type.to_string(),
        title: title.to_string(),
        content: content_value,
        repo: repo.map(|r| r.to_string()),
        status: status.map(|s| s.to_string()),
        source: source.map(|s| s.to_string()),
        tags,
        ..Default::default()
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!(
            "Recorded {} [{}]: {}",
            node.node_type,
            &node.id[..8],
            node.title
        );
        if let Some(s) = &node.status {
            println!("  Status: {}", s);
        }
    }
    Ok(())
}

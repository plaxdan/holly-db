use holly_core::{HollyDb, UpdateNodeInput};
use serde_json::Value;

#[allow(clippy::too_many_arguments)]
pub fn run(
    db: &HollyDb,
    id: &str,
    title: Option<&str>,
    content: Option<&str>,
    replace_content: bool,
    status: Option<&str>,
    repo: Option<&str>,
    tags: Option<Vec<String>>,
    json: bool,
) -> anyhow::Result<()> {
    let content_value: Option<Value> = content
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid JSON content: {}", e))?;

    let node = db.update_node(
        id,
        UpdateNodeInput {
            title: title.map(|s| s.to_string()),
            content: content_value,
            replace_content,
            status: status.map(|s| s.to_string()),
            repo: repo.map(|s| s.to_string()),
            tags,
            ..Default::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Updated [{}]: {}", &node.id[..8], node.title);
    }
    Ok(())
}

use holly_core::{HollyDb, UpdateNodeInput};

pub fn run(db: &HollyDb, id: &str, tags: Vec<String>, remove: bool, json: bool) -> anyhow::Result<()> {
    let existing = db.get_node(id)?;

    let new_tags = if remove {
        existing
            .tags
            .iter()
            .filter(|t| !tags.contains(t))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        let mut merged = existing.tags.clone();
        for tag in &tags {
            if !merged.contains(tag) {
                merged.push(tag.clone());
            }
        }
        merged
    };

    let node = db.update_node(
        id,
        UpdateNodeInput {
            tags: Some(new_tags),
            ..Default::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        let tag_display = if node.tags.is_empty() {
            "(none)".to_string()
        } else {
            node.tags.join(", ")
        };
        println!("[{}] {} — tags: {}", &node.id[..8], node.title, tag_display);
    }

    Ok(())
}

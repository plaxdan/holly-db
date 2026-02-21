use holly_core::{HollyDb, CreateNodeInput};

pub fn run(db: &HollyDb, text: &str, json: bool) -> anyhow::Result<()> {
    let node = db.create_node(CreateNodeInput {
        node_type: "memory".into(),
        title: text.to_string(),
        ..Default::default()
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("Remembered: {} [{}]", node.title, &node.id[..8]);
    }
    Ok(())
}

use holly_core::HollyDb;

pub fn run(db: &HollyDb, from: &str, to: &str, edge_type: &str, json: bool) -> anyhow::Result<()> {
    let edge = db.create_edge(from, to, edge_type, None)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&edge)?);
    } else {
        println!(
            "Connected: [{}] --[{}]--> [{}]",
            &from[..8.min(from.len())],
            edge_type,
            &to[..8.min(to.len())]
        );
    }
    Ok(())
}

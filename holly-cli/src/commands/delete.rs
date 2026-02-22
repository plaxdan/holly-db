use holly_core::HollyDb;

pub fn run(db: &HollyDb, id: &str, force: bool, json: bool) -> anyhow::Result<()> {
    if !force {
        let node = db.get_node(id)?;
        eprintln!(
            "About to delete: {} [{}] {}",
            node.node_type,
            &node.id[..8],
            node.title
        );
        eprintln!("Pass --force to confirm deletion.");
        std::process::exit(1);
    }

    db.delete_node(id)?;

    if json {
        println!("{{\"deleted\": \"{}\"}}", id);
    } else {
        println!("Deleted [{}]", &id[..8.min(id.len())]);
    }
    Ok(())
}

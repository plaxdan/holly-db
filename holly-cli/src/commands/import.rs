use holly_core::HollyDb;
use std::path::Path;

pub fn run(db: &HollyDb, from: &Path, json: bool) -> anyhow::Result<()> {
    if !from.exists() {
        return Err(anyhow::anyhow!(
            "Legacy database not found: {}",
            from.display()
        ));
    }

    eprintln!("Importing from {}...", from.display());
    let stats = db.import_from(from)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("Import complete:");
    println!(
        "  Nodes:   {} imported, {} skipped",
        stats.nodes_imported, stats.nodes_skipped
    );
    println!("  Edges:   {} imported", stats.edges_imported);
    println!("  Events:  {} imported", stats.events_imported);

    if !stats.errors.is_empty() {
        eprintln!("\nErrors ({}):", stats.errors.len());
        for e in &stats.errors {
            eprintln!("  {}", e);
        }
    }
    Ok(())
}

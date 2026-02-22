use holly_core::{embeddings, HollyDb};

pub fn run(db: &HollyDb, json: bool) -> anyhow::Result<()> {
    let model_dir = embeddings::default_model_dir();
    if !embeddings::model_available(&model_dir) {
        eprintln!("Embedding model not found. Run `holly init --download-model` first.");
        return Ok(());
    }

    if !json {
        eprintln!("Generating embeddings for nodes missing from vector index...");
    }

    let stats = db.reindex()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("Reindex complete:");
        println!("  Newly indexed:    {}", stats.indexed);
        println!("  Already indexed:  {}", stats.already_indexed);
        if !stats.errors.is_empty() {
            println!("  Errors:           {}", stats.errors.len());
            for e in &stats.errors {
                eprintln!("    {}", e);
            }
        }
    }

    Ok(())
}

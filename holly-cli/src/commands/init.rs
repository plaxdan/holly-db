use holly_core::{embeddings, HollyDb};
use std::path::Path;

pub fn run(global: bool, db_path: Option<&Path>) -> anyhow::Result<()> {
    let path = if let Some(p) = db_path {
        p.to_path_buf()
    } else if global {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
            .join(".holly-db")
            .join("holly.db")
    } else {
        std::env::current_dir()?.join(".holly-db").join("holly.db")
    };

    if path.exists() {
        println!("holly-db already initialized at {}", path.display());
        return Ok(());
    }

    let _db = HollyDb::open(&path)?;
    println!("Initialized holly-db at {}", path.display());

    // Offer to download model
    let model_dir = embeddings::default_model_dir();
    if !embeddings::model_available(&model_dir) {
        println!("\nNote: Semantic search model not found.");
        println!("Run `holly init --download-model` to enable semantic search.");
    }

    Ok(())
}

pub fn download_model() -> anyhow::Result<()> {
    let model_dir = embeddings::default_model_dir();
    println!("Downloading all-MiniLM-L6-v2 to {}...", model_dir.display());
    embeddings::download_model(&model_dir)?;
    println!("Model downloaded. Semantic search is now available.");
    Ok(())
}

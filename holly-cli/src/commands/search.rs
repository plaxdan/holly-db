use holly_core::{HollyDb, SearchOptions, embeddings};

pub fn run(
    db: &HollyDb,
    query: &str,
    node_type: Option<&str>,
    repo: Option<&str>,
    status: Option<&str>,
    semantic: bool,
    limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let opts = SearchOptions {
        node_type: node_type.map(|s| s.to_string()),
        repo: repo.map(|s| s.to_string()),
        status: status.map(|s| s.to_string()),
        limit: Some(limit),
        ..Default::default()
    };

    let results = if semantic && db.vec_available {
        let model_dir = embeddings::default_model_dir();
        if embeddings::model_available(&model_dir) {
            match embeddings::generate_embedding(query) {
                Ok(embedding) => db.hybrid_search(query, Some(&embedding), opts)?,
                Err(_) => db.fts_search(query, opts)?,
            }
        } else {
            db.fts_search(query, opts)?
        }
    } else {
        db.fts_search(query, opts)?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }

    if results.is_empty() {
        println!("No results for '{}'", query);
        return Ok(());
    }

    println!("Found {} result(s) for '{}':\n", results.len(), query);
    for r in &results {
        let n = &r.node;
        let id_short = &n.id[..8.min(n.id.len())];
        let status_str = n
            .status
            .as_deref()
            .map(|s| format!(" [{}]", s))
            .unwrap_or_default();
        let repo_str = n
            .repo
            .as_deref()
            .map(|r| format!(" ({})", r))
            .unwrap_or_default();
        println!(
            "  {} [{}] {}{}{} (score: {:.3})",
            n.node_type, id_short, n.title, status_str, repo_str, r.score
        );
    }
    Ok(())
}

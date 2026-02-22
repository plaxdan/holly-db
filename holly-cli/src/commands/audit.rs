use holly_core::{embeddings, HollyDb};

pub fn run(db: &HollyDb, fix: bool, stale_days: u32, json: bool) -> anyhow::Result<()> {
    if fix {
        // Apply fixes first so the report reflects the post-fix state.

        // 1. Orphaned edges — always safe to remove
        let removed = db.delete_orphaned_edges()?;
        if removed > 0 {
            eprintln!("Fixed: removed {} orphaned edge(s)", removed);
        }

        // 2. Missing embeddings — backfill if model is available
        let model_dir = embeddings::default_model_dir();
        if embeddings::model_available(&model_dir) {
            let stats = db.reindex()?;
            if stats.indexed > 0 {
                eprintln!("Fixed: backfilled {} missing embedding(s)", stats.indexed);
            }
        } else if !json {
            eprintln!("Note: skipping embedding backfill — model not found (run `holly init --download-model`)");
        }
    }

    let report = db.audit(stale_days)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("Audit Report");
    println!("============");
    println!("Nodes:              {}", report.total_nodes);
    println!("Edges:              {}", report.total_edges);
    println!("Events:             {}", report.total_events);
    println!("Stale nodes:        {}", report.stale_count);
    println!("Orphaned edges:     {}", report.orphaned_edges);
    println!("Missing embeddings: {}", report.missing_embeddings);
    println!("Empty content:      {}", report.empty_content_count);

    if report.stale_count > 0 {
        println!("\nStale nodes (>{} days):", stale_days);
        for n in &report.stale_nodes {
            println!(
                "  [{}] {} ({}) — {} days stale",
                &n.id[..8],
                n.title,
                n.status.as_deref().unwrap_or("?"),
                n.days_stale
            );
        }
    }

    let fixable = report.orphaned_edges + report.missing_embeddings;
    if !fix && fixable > 0 {
        println!("\n{} issue(s) can be auto-fixed. Run `holly audit --fix` to apply.", fixable);
    }

    Ok(())
}

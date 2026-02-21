use holly_core::HollyDb;

pub fn run(db: &HollyDb, fix: bool, stale_days: u32, json: bool) -> anyhow::Result<()> {
    let report = db.audit(stale_days)?;

    if fix {
        let removed = db.delete_orphaned_edges()?;
        if removed > 0 {
            eprintln!("Fixed: removed {} orphaned edge(s)", removed);
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("Holly Audit Report");
    println!("==================");
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
    Ok(())
}

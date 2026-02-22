use holly_core::HollyDb;

pub fn run(db: &HollyDb, json: bool) -> anyhow::Result<()> {
    let stats = db.stats(30)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("Holly Statistics");
    println!("================");
    println!("Total nodes:  {}", stats.total_nodes);
    println!("Total edges:  {}", stats.total_edges);
    println!("Total events: {}", stats.total_events);

    if !stats.by_type.is_empty() {
        println!("\nBy type:");
        let mut by_type: Vec<_> = stats.by_type.iter().collect();
        by_type.sort_by(|a, b| b.1.cmp(a.1));
        for (t, count) in &by_type {
            println!("  {:20} {}", t, count);
        }
    }

    if !stats.by_source.is_empty() {
        println!("\nBy source:");
        let mut by_source: Vec<_> = stats.by_source.iter().collect();
        by_source.sort_by(|a, b| b.1.cmp(a.1));
        for (s, count) in &by_source {
            println!("  {:20} {}", s, count);
        }
    }

    if !stats.daily_activity.is_empty() {
        println!("\nRecent activity:");
        for day in stats.daily_activity.iter().take(7) {
            println!("  {} — {} node(s)", day.date, day.count);
        }
    }
    Ok(())
}

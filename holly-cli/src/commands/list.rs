use holly_core::{HollyDb, ListNodesFilter};

pub fn run(
    db: &HollyDb,
    node_type: Option<&str>,
    repo: Option<&str>,
    status: Option<&str>,
    source: Option<&str>,
    limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let nodes = db.list_nodes(ListNodesFilter {
        node_type: node_type.map(|s| s.to_string()),
        repo: repo.map(|s| s.to_string()),
        status: status.map(|s| s.to_string()),
        source: source.map(|s| s.to_string()),
        limit: Some(limit),
        ..Default::default()
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&nodes)?);
        return Ok(());
    }

    if nodes.is_empty() {
        println!("No nodes found.");
        return Ok(());
    }

    println!("{} node(s):\n", nodes.len());
    for n in &nodes {
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
        println!("  {} [{}] {}{}{}", n.node_type, id_short, n.title, status_str, repo_str);
    }
    Ok(())
}

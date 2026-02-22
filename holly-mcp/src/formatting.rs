use holly_core::{AuditReport, Edge, HollyEvent, Node, SearchResult, Stats};

/// Display fields per node type (controls which content keys appear in summaries).
fn display_fields(node_type: &str) -> &'static [&'static str] {
    match node_type {
        "idea" => &["status", "source_channel", "raw_text"],
        "goal" => &["priority", "complexity", "status"],
        "requirement" => &["acceptance_criteria", "estimated_complexity"],
        "decision" => &["context", "decision", "consequences", "status", "alternatives_considered"],
        "implementation" => &["files", "commits", "status", "test_coverage"],
        "error" => &["stack_trace", "frequency", "severity", "status"],
        "improvement" => &["rationale", "impact", "effort", "status"],
        "constraint" => &["applies_to", "value", "source_file", "verified_date", "status"],
        "task" => &["status", "priority", "owner", "depends_on", "evidence"],
        "run" => &["status", "task_id", "result", "artifacts"],
        "artifact" => &["status", "artifact_type", "path", "task_id", "run_id"],
        "defect" => &["status", "severity", "category", "found_by", "origin_task_id", "origin_pr"],
        "override" => &["status", "gate", "reason", "authority", "scope", "outcome"],
        "policy" => &["status", "scope", "enforcement_level", "pass_conditions", "fail_conditions", "recovery_path", "version"],
        _ => &[],
    }
}

fn node_emoji(node_type: &str) -> &'static str {
    match node_type {
        "decision" => "⚖️",
        "error" => "🔴",
        "implementation" => "🟢",
        "improvement" => "🔄",
        "idea" => "🔵",
        "goal" => "🎯",
        "requirement" => "✅",
        _ => "📌",
    }
}

/// Format content fields for display (truncated to 200 chars for search results).
fn format_content_fields(node: &Node, truncate: bool) -> String {
    let fields = display_fields(&node.node_type);
    let mut parts = Vec::new();
    for &field in fields {
        if let Some(val) = node.content.get(field) {
            let s = match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => val.to_string(),
            };
            if !s.is_empty() {
                let display = if truncate && s.len() > 200 {
                    format!("{}…", &s[..200])
                } else {
                    s
                };
                parts.push(format!("{}: {}", field, display));
            }
        }
    }
    parts.join("\n")
}

/// Format a single node with full detail including edges.
pub fn format_node_detail(node: &Node, edges_from: &[Edge], edges_to: &[Edge]) -> String {
    let mut lines = Vec::new();

    lines.push(format!("[{}] {} ({})", node.node_type, node.title, node.id));

    if let Some(ref repo) = node.repo {
        lines.push(format!("Repo: {}", repo));
    }

    // Format timestamps to YYYY-MM-DD HH:MM
    lines.push(format!("Created: {}", format_timestamp(&node.created_at)));
    lines.push(format!("Updated: {}", format_timestamp(&node.updated_at)));

    if let Some(ref status) = node.status {
        lines.push(format!("Status: {}", status));
    }

    let content_str = format_content_fields(node, false);
    if !content_str.is_empty() {
        lines.push(content_str);
    }

    for edge in edges_from {
        let to_title = edge.to_id.chars().take(8).collect::<String>();
        lines.push(format!("Outgoing: --{}--> {} ({})", edge.edge_type, to_title, edge.to_id));
    }
    for edge in edges_to {
        let from_title = edge.from_id.chars().take(8).collect::<String>();
        lines.push(format!("Incoming: <--{}-- {} ({})", edge.edge_type, from_title, edge.from_id));
    }

    lines.join("\n")
}

/// Format a list of nodes (one-liner per node).
pub fn format_node_list(nodes: &[Node]) -> String {
    if nodes.is_empty() {
        return "No nodes found.".to_string();
    }

    let lines: Vec<String> = nodes
        .iter()
        .map(|n| {
            let status = n.status.as_deref().unwrap_or("");
            let repo = n.repo.as_deref().unwrap_or("");
            format!(
                "[{}] {} ({}){}{}",
                n.node_type,
                n.title,
                &n.id[..8.min(n.id.len())],
                if !status.is_empty() { format!(" status={}", status) } else { String::new() },
                if !repo.is_empty() { format!(" repo={}", repo) } else { String::new() },
            )
        })
        .collect();

    format!("Found {} node(s):\n\n{}", nodes.len(), lines.join("\n"))
}

/// Format a single node summary (for list contexts).
pub fn format_node_summary(node: &Node) -> String {
    let status = node.status.as_deref().unwrap_or("");
    format!(
        "[{}] {} ({}){}",
        node.node_type,
        node.title,
        &node.id[..8.min(node.id.len())],
        if !status.is_empty() { format!(" status={}", status) } else { String::new() },
    )
}

/// Format search results with scores.
pub fn format_search_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }

    let mut lines = vec![format!("Found {} result(s):\n", results.len())];
    for r in results {
        let score_line = format!(
            "[{}] {} ({})  score={:.3}",
            r.node.node_type,
            r.node.title,
            &r.node.id[..8.min(r.node.id.len())],
            r.score
        );
        lines.push(score_line);
        let fields = format_content_fields(&r.node, true);
        if !fields.is_empty() {
            lines.push(fields);
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Format nodes as a markdown recent-activity table grouped by date.
pub fn format_recent_table(nodes: &[Node]) -> String {
    if nodes.is_empty() {
        return "No recent activity.".to_string();
    }

    let mut lines = vec![
        "| ID | Time | T | Title | Source | Repo |".to_string(),
        "|---|---|---|---|---|---|".to_string(),
    ];

    let mut current_date = String::new();

    for node in nodes {
        // Extract date from ISO8601 (chars 0..10)
        let date = if node.updated_at.len() >= 10 { &node.updated_at[..10] } else { &node.updated_at };
        // Extract time HH:MM from chars 11..16
        let time = if node.updated_at.len() >= 16 { &node.updated_at[11..16] } else { "" };

        if date != current_date {
            current_date = date.to_string();
        }

        let short_id = &node.id[..8.min(node.id.len())];
        let emoji = node_emoji(&node.node_type);
        let repo = node.repo.as_deref().unwrap_or("");

        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            short_id, time, emoji, node.title, node.source, repo
        ));
    }

    lines.join("\n")
}

/// Format an edge for display.
pub fn format_edge(edge: &Edge, from_title: &str, to_title: &str) -> String {
    format!(
        "{} ({}) --{}--> {} ({})",
        from_title, &edge.from_id[..8.min(edge.from_id.len())],
        edge.edge_type,
        to_title, &edge.to_id[..8.min(edge.to_id.len())]
    )
}

/// Format event list.
pub fn format_event_list(events: &[HollyEvent]) -> String {
    if events.is_empty() {
        return "No events found.".to_string();
    }

    let lines: Vec<String> = events
        .iter()
        .map(|e| {
            format!(
                "[{}] {} ({}){}{}",
                format_timestamp(&e.created_at),
                e.event_type,
                &e.id[..8.min(e.id.len())],
                e.repo.as_ref().map_or(String::new(), |r| format!(" repo={}", r)),
                e.workspace.as_ref().map_or(String::new(), |w| format!(" workspace={}", w)),
            )
        })
        .collect();

    format!("Found {} event(s):\n\n{}", events.len(), lines.join("\n"))
}

/// Format stats report.
pub fn format_stats(stats: &Stats) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Total nodes:  {}", stats.total_nodes));
    lines.push(format!("Total edges:  {}", stats.total_edges));
    lines.push(format!("Total events: {}", stats.total_events));

    if !stats.by_type.is_empty() {
        lines.push("\nBy type:".to_string());
        let mut by_type: Vec<_> = stats.by_type.iter().collect();
        by_type.sort_by(|a, b| b.1.cmp(a.1));
        for (t, count) in &by_type {
            lines.push(format!("  {:20} {}", t, count));
        }
    }

    if !stats.by_source.is_empty() {
        lines.push("\nBy source:".to_string());
        let mut by_src: Vec<_> = stats.by_source.iter().collect();
        by_src.sort_by(|a, b| b.1.cmp(a.1));
        for (s, count) in &by_src {
            lines.push(format!("  {:20} {}", s, count));
        }
    }

    if !stats.daily_activity.is_empty() {
        lines.push("\nRecent activity:".to_string());
        for day in stats.daily_activity.iter().take(7) {
            lines.push(format!("  {} — {} node(s)", day.date, day.count));
        }
    }

    lines.join("\n")
}

/// Format audit report.
pub fn format_audit(report: &AuditReport, mode: &str) -> String {
    let mut lines = Vec::new();

    lines.push(format!("Total nodes: {}", report.total_nodes));
    lines.push(format!("Total edges: {}", report.total_edges));
    lines.push(format!("Total events: {}", report.total_events));
    lines.push(format!("Stale nodes: {}", report.stale_count));
    lines.push(format!("Orphaned edges: {}", report.orphaned_edges));
    lines.push(format!("Missing embeddings: {}", report.missing_embeddings));
    lines.push(format!("Empty content: {}", report.empty_content_count));
    lines.push(format!("Similarity/duplicate detection: not yet implemented"));

    if mode == "detail" && !report.stale_nodes.is_empty() {
        lines.push("\nStale nodes:".to_string());
        for n in &report.stale_nodes {
            lines.push(format!(
                "  [{}] {} ({}) — {} days stale, status={}",
                n.node_type,
                n.title,
                &n.id[..8.min(n.id.len())],
                n.days_stale,
                n.status.as_deref().unwrap_or("none"),
            ));
        }
    }

    lines.join("\n")
}

fn format_timestamp(ts: &str) -> String {
    // ISO8601: 2024-01-15T10:30:00Z → 2024-01-15 10:30
    if ts.len() >= 16 {
        format!("{} {}", &ts[..10], &ts[11..16])
    } else {
        ts.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use holly_core::{Node, SearchResult};

    fn make_node(id: &str, node_type: &str, title: &str) -> Node {
        Node {
            id: id.to_string(),
            node_type: node_type.to_string(),
            title: title.to_string(),
            content: serde_json::json!({"status": "open", "context": "test context"}),
            tags: vec![],
            repo: None,
            status: Some("open".to_string()),
            source: "curated".to_string(),
            agent: None,
            user: None,
            llm: None,
            created_at: "2024-01-15T10:30:00Z".to_string(),
            updated_at: "2024-01-16T14:20:00Z".to_string(),
        }
    }

    #[test]
    fn test_format_node_detail() {
        let node = make_node("abc123def456", "decision", "Use SQLite");
        let result = format_node_detail(&node, &[], &[]);
        assert!(result.contains("[decision] Use SQLite"));
        assert!(result.contains("abc123def456"));
        assert!(result.contains("Created: 2024-01-15 10:30"));
    }

    #[test]
    fn test_format_search_results_score() {
        let node = make_node("abc123def456", "decision", "Test");
        let results = vec![SearchResult { node, score: 0.123456 }];
        let s = format_search_results(&results);
        assert!(s.contains("score=0.123"));
        assert!(!s.contains("score=0.1234")); // only 3 decimal places
    }

    #[test]
    fn test_format_recent_table_structure() {
        let node = make_node("abc123def456", "decision", "Test node");
        let s = format_recent_table(&[node]);
        assert!(s.contains("| ID | Time | T | Title | Source | Repo |"));
        assert!(s.contains("abc123de")); // 8-char ID
    }

    #[test]
    fn test_format_empty_results() {
        assert_eq!(format_search_results(&[]), "No results found.");
        assert_eq!(format_node_list(&[]), "No nodes found.");
        assert_eq!(format_recent_table(&[]), "No recent activity.");
    }
}

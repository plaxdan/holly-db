use crate::db::HollyDb;
use crate::error::Result;
use crate::nodes::{ListNodesFilter, Node};

/// Context export output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextFormat {
    Markdown,
    Json,
}

/// Context export result.
#[derive(Debug, serde::Serialize)]
pub struct ContextExport {
    pub constraints: Vec<Node>,
    pub decisions: Vec<Node>,
    pub in_progress_tasks: Vec<Node>,
    pub recent_errors: Vec<Node>,
    pub pinned: Vec<Node>,
}

impl HollyDb {
    /// Export active context for consumption by agents.
    pub fn export_context(&self) -> Result<ContextExport> {
        let constraints = self.list_nodes(ListNodesFilter {
            node_type: Some("constraint".into()),
            status: Some("active".into()),
            limit: Some(20),
            ..Default::default()
        })?;

        let decisions = self.list_nodes(ListNodesFilter {
            node_type: Some("decision".into()),
            status: Some("accepted".into()),
            limit: Some(20),
            ..Default::default()
        })?;

        let in_progress_tasks = self.list_nodes(ListNodesFilter {
            node_type: Some("task".into()),
            status: Some("in_progress".into()),
            limit: Some(10),
            ..Default::default()
        })?;

        let recent_errors = self.list_nodes(ListNodesFilter {
            node_type: Some("error".into()),
            status: Some("open".into()),
            limit: Some(10),
            ..Default::default()
        })?;

        Ok(ContextExport {
            constraints,
            decisions,
            in_progress_tasks,
            recent_errors,
            pinned: Vec::new(), // Future: pinned nodes support
        })
    }

    /// Render context as markdown.
    pub fn export_context_markdown(&self) -> Result<String> {
        let ctx = self.export_context()?;
        let mut out = String::new();

        out.push_str("# Holly Context\n\n");

        if !ctx.constraints.is_empty() {
            out.push_str("## Active Constraints\n\n");
            for n in &ctx.constraints {
                out.push_str(&format!("- **{}** ({})\n", n.title, n.node_type));
                if let Some(applies_to) = n.content.get("applies_to").and_then(|v| v.as_str()) {
                    if !applies_to.is_empty() {
                        out.push_str(&format!("  - Applies to: {}\n", applies_to));
                    }
                }
                if let Some(value) = n.content.get("value").and_then(|v| v.as_str()) {
                    if !value.is_empty() {
                        out.push_str(&format!("  - Value: {}\n", value));
                    }
                }
            }
            out.push('\n');
        }

        if !ctx.decisions.is_empty() {
            out.push_str("## Accepted Decisions\n\n");
            for n in &ctx.decisions {
                out.push_str(&format!("- **{}**\n", n.title));
                if let Some(decision) = n.content.get("decision").and_then(|v| v.as_str()) {
                    if !decision.is_empty() {
                        out.push_str(&format!("  - {}\n", decision));
                    }
                }
            }
            out.push('\n');
        }

        if !ctx.in_progress_tasks.is_empty() {
            out.push_str("## In-Progress Tasks\n\n");
            for n in &ctx.in_progress_tasks {
                out.push_str(&format!("- **{}**\n", n.title));
            }
            out.push('\n');
        }

        if !ctx.recent_errors.is_empty() {
            out.push_str("## Open Errors\n\n");
            for n in &ctx.recent_errors {
                out.push_str(&format!("- **{}**\n", n.title));
            }
            out.push('\n');
        }

        if out == "# Holly Context\n\n" {
            out.push_str("*No active context found.*\n");
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::HollyDb;
    use crate::nodes::CreateNodeInput;

    #[test]
    fn test_export_context_empty() {
        let db = HollyDb::open_in_memory().unwrap();
        let ctx = db.export_context().unwrap();
        assert!(ctx.constraints.is_empty());
        assert!(ctx.decisions.is_empty());
    }

    #[test]
    fn test_export_context_with_data() {
        let db = HollyDb::open_in_memory().unwrap();
        db.create_node(CreateNodeInput {
            node_type: "constraint".into(),
            title: "Java 17 required".into(),
            status: Some("active".into()),
            ..Default::default()
        })
        .unwrap();

        let ctx = db.export_context().unwrap();
        assert_eq!(ctx.constraints.len(), 1);
    }

    #[test]
    fn test_export_context_markdown() {
        let db = HollyDb::open_in_memory().unwrap();
        db.create_node(CreateNodeInput {
            node_type: "constraint".into(),
            title: "Rust 2021 edition".into(),
            status: Some("active".into()),
            ..Default::default()
        })
        .unwrap();

        let md = db.export_context_markdown().unwrap();
        assert!(md.contains("# Holly Context"));
        assert!(md.contains("Rust 2021 edition"));
    }
}

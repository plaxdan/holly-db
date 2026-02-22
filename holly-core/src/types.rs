use crate::error::{HollyError, Result};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Core node types built into holly-db.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Memory,
    Decision,
    Idea,
    Error,
    Constraint,
    Implementation,
    Improvement,
    Task,
    Run,
    Artifact,
    Goal,
    /// Custom type defined in holly.config.yaml
    Custom(String),
}

impl NodeType {
    pub fn as_str(&self) -> &str {
        match self {
            NodeType::Memory => "memory",
            NodeType::Decision => "decision",
            NodeType::Idea => "idea",
            NodeType::Error => "error",
            NodeType::Constraint => "constraint",
            NodeType::Implementation => "implementation",
            NodeType::Improvement => "improvement",
            NodeType::Task => "task",
            NodeType::Run => "run",
            NodeType::Artifact => "artifact",
            NodeType::Goal => "goal",
            NodeType::Custom(s) => s.as_str(),
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, NodeType::Custom(_))
    }

    pub fn all_core() -> &'static [&'static str] {
        &[
            "memory",
            "decision",
            "idea",
            "error",
            "constraint",
            "implementation",
            "improvement",
            "task",
            "run",
            "artifact",
            "goal",
        ]
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for NodeType {
    type Err = HollyError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "memory" => Ok(NodeType::Memory),
            "decision" => Ok(NodeType::Decision),
            "idea" => Ok(NodeType::Idea),
            "error" => Ok(NodeType::Error),
            "constraint" => Ok(NodeType::Constraint),
            "implementation" => Ok(NodeType::Implementation),
            "improvement" => Ok(NodeType::Improvement),
            "task" => Ok(NodeType::Task),
            "run" => Ok(NodeType::Run),
            "artifact" => Ok(NodeType::Artifact),
            "goal" => Ok(NodeType::Goal),
            other => Ok(NodeType::Custom(other.to_string())),
        }
    }
}

/// Core edge types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    RelatesTo,
    DerivesFrom,
    Implements,
    Blocks,
    CausedBy,
    Fixes,
    Supersedes,
    /// Custom edge type
    Custom(String),
}

impl EdgeType {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeType::RelatesTo => "relates_to",
            EdgeType::DerivesFrom => "derives_from",
            EdgeType::Implements => "implements",
            EdgeType::Blocks => "blocks",
            EdgeType::CausedBy => "caused_by",
            EdgeType::Fixes => "fixes",
            EdgeType::Supersedes => "supersedes",
            EdgeType::Custom(s) => s.as_str(),
        }
    }
}

impl fmt::Display for EdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for EdgeType {
    type Err = HollyError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "relates_to" => Ok(EdgeType::RelatesTo),
            "derives_from" => Ok(EdgeType::DerivesFrom),
            "implements" => Ok(EdgeType::Implements),
            "blocks" => Ok(EdgeType::Blocks),
            "caused_by" => Ok(EdgeType::CausedBy),
            "fixes" => Ok(EdgeType::Fixes),
            "supersedes" => Ok(EdgeType::Supersedes),
            other => Ok(EdgeType::Custom(other.to_string())),
        }
    }
}

/// Per-type status allowlists.
pub fn status_allowlist(node_type: &str) -> Option<&'static [&'static str]> {
    match node_type {
        "idea" => Some(&[
            "open",
            "researching",
            "research-complete",
            "implementing",
            "implemented",
            "abandoned",
        ]),
        "goal" => Some(&["planning", "active", "blocked", "completed"]),
        "decision" => Some(&["proposed", "accepted", "deprecated", "superseded"]),
        "implementation" => Some(&[
            "planned",
            "in_progress",
            "blocked",
            "completed",
            "implemented",
        ]),
        "error" => Some(&["open", "investigating", "resolved"]),
        "improvement" => Some(&["proposed", "accepted", "implemented", "deprecated"]),
        "constraint" => Some(&["active", "deprecated"]),
        "task" => Some(&[
            "planned",
            "in_progress",
            "blocked",
            "completed",
            "cancelled",
        ]),
        "run" => Some(&[
            "started",
            "in_progress",
            "completed",
            "failed",
            "aborted",
        ]),
        "artifact" => Some(&["recorded", "verified", "deprecated"]),
        "memory" => None,
        _ => None,
    }
}

/// Default status per node type.
pub fn default_status(node_type: &str) -> Option<&'static str> {
    match node_type {
        "idea" => Some("open"),
        "goal" => Some("planning"),
        "decision" => Some("proposed"),
        "implementation" => Some("in_progress"),
        "error" => Some("open"),
        "improvement" => Some("proposed"),
        "constraint" => Some("active"),
        "task" => Some("planned"),
        "run" => Some("started"),
        "artifact" => Some("recorded"),
        _ => None,
    }
}

/// Valid transitions per node type (from → allowed_to).
pub fn valid_transitions(node_type: &str) -> HashMap<&'static str, &'static [&'static str]> {
    let mut map = HashMap::new();
    match node_type {
        "task" => {
            map.insert("planned", ["in_progress", "blocked", "cancelled"].as_slice());
            map.insert("in_progress", ["completed", "blocked", "cancelled"].as_slice());
            map.insert("blocked", ["in_progress", "cancelled"].as_slice());
            map.insert("completed", [].as_slice());
            map.insert("cancelled", [].as_slice());
        }
        "run" => {
            map.insert("started", ["in_progress", "completed", "failed", "aborted"].as_slice());
            map.insert("in_progress", ["completed", "failed", "aborted"].as_slice());
            map.insert("completed", [].as_slice());
            map.insert("failed", [].as_slice());
            map.insert("aborted", [].as_slice());
        }
        _ => {}
    }
    map
}

/// Normalize a status string: lowercase, trim, replace spaces/dashes with underscores where needed.
pub fn normalize_status(node_type: &str, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let Some(allowed) = status_allowlist(node_type) else {
        // No allowlist — return as-is
        return Some(trimmed.to_string());
    };

    // Try direct match first
    let lower = trimmed.to_lowercase();
    if allowed.contains(&lower.as_str()) {
        return Some(lower);
    }

    // Try with spaces → underscores
    let underscored = lower.replace([' ', '-'], "_");
    if allowed.contains(&underscored.as_str()) {
        return Some(underscored);
    }

    // Try with spaces → dashes
    let dashed = lower.replace(' ', "-");
    if allowed.contains(&dashed.as_str()) {
        return Some(dashed);
    }

    // Alias table: (prefix, target)
    let aliases: &[(&str, &str)] = match node_type {
        "constraint" => &[
            ("completed", "active"),
            ("implemented", "active"),
            ("resolved", "active"),
        ],
        "implementation" => &[("in progress", "in_progress"), ("in_progress", "in_progress")],
        "task" => &[("in progress", "in_progress"), ("in_progress", "in_progress")],
        "run" => &[("in progress", "in_progress"), ("in_progress", "in_progress")],
        _ => &[],
    };

    for (prefix, target) in aliases {
        if lower.starts_with(prefix) {
            return Some(target.to_string());
        }
    }

    None
}

/// Apply status governance: validate or normalize the status field.
/// Returns error if strict mode and status is invalid.
pub fn apply_status_governance(
    node_type: &str,
    status: Option<&str>,
    strict: bool,
) -> Result<Option<String>> {
    let Some(raw) = status else {
        // No status provided — use default
        return Ok(default_status(node_type).map(|s| s.to_string()));
    };

    if let Some(normalized) = normalize_status(node_type, raw) {
        return Ok(Some(normalized));
    }

    let Some(allowed) = status_allowlist(node_type) else {
        return Ok(Some(raw.to_string()));
    };

    if strict {
        return Err(HollyError::InvalidStatus {
            status: raw.to_string(),
            node_type: node_type.to_string(),
            allowed: allowed.join(", "),
        });
    }

    // Normalize mode — fall back to default
    Ok(default_status(node_type).map(|s| s.to_string()))
}

/// Validate a status transition for governed types (task, run).
pub fn validate_transition(node_type: &str, from: &str, to: &str) -> Result<()> {
    let transitions = valid_transitions(node_type);
    if transitions.is_empty() {
        return Ok(());
    }

    let allowed = transitions.get(from).copied().unwrap_or(&[]);
    if allowed.contains(&to) || allowed.is_empty() && from == to {
        return Ok(());
    }

    Err(HollyError::InvalidTransition {
        from: from.to_string(),
        to: to.to_string(),
        node_type: node_type.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_type_from_str() {
        assert_eq!(NodeType::from_str("decision").unwrap(), NodeType::Decision);
        assert_eq!(NodeType::from_str("TASK").unwrap(), NodeType::Task);
        assert_eq!(
            NodeType::from_str("custom_type").unwrap(),
            NodeType::Custom("custom_type".into())
        );
    }

    #[test]
    fn test_normalize_status() {
        assert_eq!(
            normalize_status("task", "in progress"),
            Some("in_progress".into())
        );
        assert_eq!(
            normalize_status("task", "in_progress"),
            Some("in_progress".into())
        );
        assert_eq!(normalize_status("task", "planned"), Some("planned".into()));
        assert_eq!(normalize_status("task", "invalid_xyz"), None);
    }

    #[test]
    fn test_normalize_status_constraint_aliases() {
        assert_eq!(
            normalize_status("constraint", "completed"),
            Some("active".into())
        );
        assert_eq!(
            normalize_status("constraint", "implemented"),
            Some("active".into())
        );
    }

    #[test]
    fn test_status_governance_strict() {
        let result = apply_status_governance("task", Some("bad_status"), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_status_governance_default() {
        let result = apply_status_governance("task", None, true).unwrap();
        assert_eq!(result, Some("planned".into()));
    }

    #[test]
    fn test_transition_task() {
        assert!(validate_transition("task", "planned", "in_progress").is_ok());
        assert!(validate_transition("task", "planned", "completed").is_err());
        assert!(validate_transition("task", "completed", "in_progress").is_err());
    }

    #[test]
    fn test_transition_no_governance() {
        // Non-governed types allow any transition
        assert!(validate_transition("idea", "open", "anything").is_ok());
    }
}

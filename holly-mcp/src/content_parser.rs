use serde_json::{Map, Value};

/// Parse free-form content text into a structured JSON object for the given node type.
pub fn parse_content(node_type: &str, text: &str) -> Value {
    match node_type {
        "decision" => parse_decision(text),
        "goal" => parse_labeled(
            text,
            &["priority", "complexity", "status"],
            &[],
            "description",
        ),
        "error" => parse_labeled(
            text,
            &["stack_trace", "severity", "status", "frequency"],
            &[],
            "stack_trace",
        ),
        "improvement" => parse_labeled(
            text,
            &["rationale", "impact", "effort", "status"],
            &[],
            "rationale",
        ),
        "idea" => parse_idea(text),
        "requirement" => parse_labeled(
            text,
            &["acceptance_criteria", "estimated_complexity"],
            &["acceptance_criteria"],
            "description",
        ),
        "implementation" => parse_labeled(
            text,
            &["files", "commits", "status", "test_coverage"],
            &["files", "commits"],
            "description",
        ),
        "constraint" => parse_labeled(
            text,
            &[
                "applies_to",
                "value",
                "source_file",
                "verified_date",
                "status",
            ],
            &[],
            "description",
        ),
        "task" => parse_labeled(
            text,
            &["status", "priority", "owner", "depends_on", "evidence"],
            &["depends_on", "evidence"],
            "description",
        ),
        "run" => parse_labeled(
            text,
            &["status", "task_id", "result", "artifacts"],
            &["artifacts"],
            "description",
        ),
        "artifact" => parse_labeled(
            text,
            &["status", "artifact_type", "path", "task_id", "run_id"],
            &[],
            "description",
        ),
        "defect" => parse_labeled(
            text,
            &[
                "origin_task_id",
                "origin_pr",
                "found_by",
                "severity",
                "category",
                "status",
            ],
            &[],
            "description",
        ),
        "override" => parse_labeled(
            text,
            &["gate", "reason", "authority", "scope", "outcome", "status"],
            &[],
            "description",
        ),
        "policy" => parse_labeled(
            text,
            &[
                "scope",
                "enforcement_level",
                "pass_conditions",
                "fail_conditions",
                "recovery_path",
                "version",
                "status",
            ],
            &["pass_conditions", "fail_conditions"],
            "description",
        ),
        _ => Value::Object({
            let mut m = Map::new();
            m.insert("text".into(), Value::String(text.to_string()));
            m
        }),
    }
}

/// Extract the Status: line from content text (for promoting to node top-level status).
pub fn extract_status(text: &str) -> Option<String> {
    for line in text.lines() {
        let lower = line.to_lowercase();
        if let Some(rest) = lower.strip_prefix("status:") {
            let status = rest.trim().to_string();
            if !status.is_empty() {
                // Return from original (not lowercased) to preserve case
                let original_rest = &line[line.to_lowercase().find("status:").unwrap() + 7..];
                return Some(original_rest.trim().to_string());
            }
        }
    }
    None
}

fn parse_labeled(
    text: &str,
    fields: &[&str],
    array_fields: &[&str],
    fallback_field: &str,
) -> Value {
    let mut obj = Map::new();

    for field in fields {
        let pattern_lower = format!("{}:", field.to_lowercase());
        for line in text.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.trim_start().starts_with(&pattern_lower) {
                // Find the colon in the original line
                if let Some(colon_pos) = line.find(':') {
                    let value_str = line[colon_pos + 1..].trim().to_string();
                    if !value_str.is_empty() {
                        if array_fields.contains(field) {
                            let parts: Vec<Value> = value_str
                                .split(',')
                                .map(|s| Value::String(s.trim().to_string()))
                                .filter(|v| v.as_str().is_some_and(|s| !s.is_empty()))
                                .collect();
                            obj.insert(field.to_string(), Value::Array(parts));
                        } else {
                            // numeric fields
                            if *field == "priority" || *field == "frequency" {
                                if let Ok(n) = value_str.parse::<i64>() {
                                    obj.insert(field.to_string(), Value::Number(n.into()));
                                    break;
                                }
                            }
                            obj.insert(field.to_string(), Value::String(value_str));
                        }
                        break;
                    }
                }
            }
        }
    }

    // If no fields matched, store whole text in fallback field
    if obj.is_empty() {
        obj.insert(fallback_field.to_string(), Value::String(text.to_string()));
    }

    Value::Object(obj)
}

fn parse_decision(text: &str) -> Value {
    let mut obj = Map::new();

    // Try section-based parsing first
    let section_names = [
        "Context",
        "Decision",
        "Consequences",
        "Alternatives",
        "Status",
    ];
    let field_keys = [
        "context",
        "decision",
        "consequences",
        "alternatives_considered",
        "status",
    ];

    // Split on section headers
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_content: Vec<String> = Vec::new();

    for line in text.lines() {
        let line_trimmed = line.trim();
        let mut found_section = false;
        for name in &section_names {
            let header = format!("{}:", name);
            if line_trimmed.eq_ignore_ascii_case(&header) || line_trimmed.starts_with(&header) {
                if let Some(sec) = current_section.take() {
                    sections.push((sec, current_content.join("\n").trim().to_string()));
                    current_content.clear();
                }
                current_section = Some(name.to_lowercase().to_string());
                // If there's content on the same line after the colon
                let after_colon = line_trimmed[header.len()..].trim().to_string();
                if !after_colon.is_empty() {
                    current_content.push(after_colon);
                }
                found_section = true;
                break;
            }
        }
        if !found_section && current_section.is_some() {
            current_content.push(line.to_string());
        }
    }
    if let Some(sec) = current_section {
        sections.push((sec, current_content.join("\n").trim().to_string()));
    }

    if !sections.is_empty() {
        for (sec, content) in &sections {
            if sec == "alternatives" {
                let parts: Vec<Value> = content
                    .split(',')
                    .map(|s| Value::String(s.trim().to_string()))
                    .filter(|v| v.as_str().is_some_and(|s| !s.is_empty()))
                    .collect();
                obj.insert("alternatives_considered".to_string(), Value::Array(parts));
            } else {
                for (idx, name) in section_names.iter().enumerate() {
                    if sec == &name.to_lowercase() {
                        obj.insert(field_keys[idx].to_string(), Value::String(content.clone()));
                        break;
                    }
                }
            }
        }
    } else {
        // Fall back to line-by-line parsing
        obj = parse_labeled(
            text,
            &["context", "decision", "consequences", "status"],
            &["alternatives_considered"],
            "context",
        )
        .as_object()
        .cloned()
        .unwrap_or_default();
    }

    Value::Object(obj)
}

fn parse_idea(text: &str) -> Value {
    let mut obj = Map::new();

    // Try to extract status
    for line in text.lines() {
        let lower = line.to_lowercase();
        if let Some(rest) = lower.strip_prefix("status:") {
            let raw = rest.trim().to_string();
            // normalize spaces to hyphens
            let normalized = raw.replace(' ', "-");
            obj.insert("status".to_string(), Value::String(normalized));
        }
    }

    // Always set source_channel for ideas
    obj.insert(
        "source_channel".to_string(),
        Value::String("claude_code".to_string()),
    );

    // If no status found, store as raw_text
    if !obj.contains_key("status") {
        obj.insert("raw_text".to_string(), Value::String(text.to_string()));
    } else {
        // Store remaining content as raw_text
        let raw: String = text
            .lines()
            .filter(|l| !l.to_lowercase().starts_with("status:"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        if !raw.is_empty() {
            obj.insert("raw_text".to_string(), Value::String(raw));
        }
    }

    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decision_sections() {
        let text = "Context: We needed to choose a storage engine\nDecision: Use SQLite\nConsequences: Simple setup\nStatus: accepted";
        let val = parse_content("decision", text);
        let obj = val.as_object().unwrap();
        assert_eq!(
            obj["context"].as_str().unwrap(),
            "We needed to choose a storage engine"
        );
        assert_eq!(obj["decision"].as_str().unwrap(), "Use SQLite");
        assert_eq!(obj["status"].as_str().unwrap(), "accepted");
    }

    #[test]
    fn test_parse_goal_priority() {
        let text = "Priority: 8\nComplexity: high\nStatus: active";
        let val = parse_content("goal", text);
        let obj = val.as_object().unwrap();
        assert_eq!(obj["priority"].as_i64().unwrap(), 8);
        assert_eq!(obj["complexity"].as_str().unwrap(), "high");
    }

    #[test]
    fn test_parse_implementation_arrays() {
        let text = "Files: src/main.rs, src/lib.rs\nCommits: abc123, def456\nStatus: completed";
        let val = parse_content("implementation", text);
        let obj = val.as_object().unwrap();
        let files = obj["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].as_str().unwrap(), "src/main.rs");
    }

    #[test]
    fn test_parse_idea_status_normalized() {
        let text = "Status: in progress\nSome idea content";
        let val = parse_content("idea", text);
        let obj = val.as_object().unwrap();
        assert_eq!(obj["status"].as_str().unwrap(), "in-progress");
        assert_eq!(obj["source_channel"].as_str().unwrap(), "claude_code");
    }

    #[test]
    fn test_parse_unknown_type() {
        let text = "some random text";
        let val = parse_content("unknown", text);
        let obj = val.as_object().unwrap();
        assert_eq!(obj["text"].as_str().unwrap(), "some random text");
    }

    #[test]
    fn test_parse_fallback_empty_match() {
        let text = "This has no labeled fields at all";
        let val = parse_content("constraint", text);
        let obj = val.as_object().unwrap();
        // Falls back to description field
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn test_extract_status() {
        let text = "Context: something\nStatus: accepted\nMore text";
        assert_eq!(extract_status(text), Some("accepted".to_string()));

        let text_no_status = "No status here";
        assert_eq!(extract_status(text_no_status), None);
    }
}

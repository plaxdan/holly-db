/// Provenance fields stamped on every write.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Provenance {
    pub agent: Option<String>,
    pub user: Option<String>,
    pub llm: Option<String>,
}

impl Provenance {
    /// Detect provenance from environment variables.
    ///
    /// Agent detection order:
    /// 1. HOLLY_AGENT
    /// 2. CURSOR_INVOKED_AS
    /// 3. CURSOR_AGENT=1 → "cursor-agent"
    /// 4. CLAUDE_CODE_AGENT
    /// 5. CLAUDE_PROJECT_DIR set → "claude-code"
    /// 6. None
    ///
    /// LLM detection order:
    /// 1. HOLLY_LLM
    /// 2. CURSOR_MODEL / CURSOR_LLM
    /// 3. CLAUDE_MODEL / ANTHROPIC_MODEL
    /// 4. OPENAI_MODEL / MODEL_NAME / LLM_MODEL / MODEL
    /// 5. None
    pub fn from_env() -> Self {
        Provenance {
            agent: detect_agent(),
            user: std::env::var("HOLLY_USER").ok(),
            llm: detect_llm(),
        }
    }

    /// Merge with another provenance, preferring self's non-None values.
    pub fn merge(self, other: Provenance) -> Provenance {
        Provenance {
            agent: self.agent.or(other.agent),
            user: self.user.or(other.user),
            llm: self.llm.or(other.llm),
        }
    }
}

fn detect_agent() -> Option<String> {
    if let Ok(v) = std::env::var("HOLLY_AGENT") {
        return Some(v);
    }
    if let Ok(v) = std::env::var("CURSOR_INVOKED_AS") {
        return Some(v);
    }
    if std::env::var("CURSOR_AGENT").as_deref() == Ok("1") {
        return Some("cursor-agent".into());
    }
    if let Ok(v) = std::env::var("CLAUDE_CODE_AGENT") {
        return Some(v);
    }
    if std::env::var("CLAUDE_PROJECT_DIR").is_ok() {
        return Some("claude-code".into());
    }
    None
}

fn detect_llm() -> Option<String> {
    for var in &[
        "HOLLY_LLM",
        "CURSOR_MODEL",
        "CURSOR_LLM",
        "CLAUDE_MODEL",
        "ANTHROPIC_MODEL",
        "OPENAI_MODEL",
        "MODEL_NAME",
        "LLM_MODEL",
        "MODEL",
    ] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_from_env_empty() {
        // Without env vars set, all fields are None
        let p = Provenance {
            agent: None,
            user: None,
            llm: None,
        };
        assert!(p.agent.is_none());
        assert!(p.user.is_none());
        assert!(p.llm.is_none());
    }

    #[test]
    fn test_provenance_merge() {
        let a = Provenance {
            agent: Some("claude-code".into()),
            user: None,
            llm: None,
        };
        let b = Provenance {
            agent: Some("cursor".into()),
            user: Some("alice".into()),
            llm: Some("gpt-4".into()),
        };
        let merged = a.merge(b);
        assert_eq!(merged.agent.as_deref(), Some("claude-code"));
        assert_eq!(merged.user.as_deref(), Some("alice"));
        assert_eq!(merged.llm.as_deref(), Some("gpt-4"));
    }
}

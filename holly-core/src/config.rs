use crate::error::{HollyError, Result};
use std::collections::HashMap;
use std::path::Path;

/// Custom type definition from holly.config.yaml.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomType {
    pub name: String,
    pub statuses: Option<Vec<String>>,
    pub default_status: Option<String>,
    pub default_content: Option<serde_json::Value>,
}

/// Holly configuration (holly.config.yaml).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HollyConfig {
    /// Custom node types (cannot override core types).
    #[serde(default)]
    pub types: Vec<CustomType>,
    /// Custom edge types.
    #[serde(default)]
    pub edge_types: Vec<String>,
}

impl HollyConfig {
    /// Load config from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: HollyConfig = serde_yaml::from_str(&content)
            .map_err(|e| HollyError::Config(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Load config from the auto-discovered location:
    /// 1. HOLLY_CONFIG env var
    /// 2. Walk up from cwd for holly.config.yaml
    /// 3. ~/.holly-db/holly.config.yaml
    pub fn discover() -> Option<Self> {
        if let Ok(path) = std::env::var("HOLLY_CONFIG") {
            return Self::from_file(Path::new(&path)).ok();
        }

        if let Ok(cwd) = std::env::current_dir() {
            let mut dir = cwd.as_path();
            loop {
                let candidate = dir.join("holly.config.yaml");
                if candidate.exists() {
                    return Self::from_file(&candidate).ok();
                }
                match dir.parent() {
                    Some(p) => dir = p,
                    None => break,
                }
            }
        }

        if let Some(home) = dirs::home_dir() {
            let global = home.join(".holly-db").join("holly.config.yaml");
            if global.exists() {
                return Self::from_file(&global).ok();
            }
        }

        None
    }

    /// Validate that custom types don't override core types.
    fn validate(&self) -> Result<()> {
        use crate::types::NodeType;
        for ct in &self.types {
            for core in NodeType::all_core() {
                if ct.name == *core {
                    return Err(HollyError::Config(format!(
                        "Custom type '{}' cannot override a core type",
                        ct.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Build a map of custom type name → CustomType.
    pub fn custom_types_map(&self) -> HashMap<String, &CustomType> {
        self.types.iter().map(|t| (t.name.clone(), t)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_config() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
types:
  - name: spike
    statuses: [open, in_progress, done]
    default_status: open
edge_types:
  - motivates
"#
        )
        .unwrap();

        let config = HollyConfig::from_file(f.path()).unwrap();
        assert_eq!(config.types.len(), 1);
        assert_eq!(config.types[0].name, "spike");
        assert_eq!(config.edge_types, vec!["motivates"]);
    }

    #[test]
    fn test_reject_core_type_override() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
types:
  - name: decision
    statuses: [open]
"#
        )
        .unwrap();

        let result = HollyConfig::from_file(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_config() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f).unwrap();
        let config = HollyConfig::from_file(f.path()).unwrap();
        assert!(config.types.is_empty());
    }
}

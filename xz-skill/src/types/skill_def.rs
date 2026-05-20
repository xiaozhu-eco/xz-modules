use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::SkillError;

/// Definition of a skill loaded from a SKILL.md file.
///
/// This is a lightweight type that represents the parsed YAML frontmatter
/// of a SKILL.md file. It contains the skill's metadata and references to
/// its WASM binary and available tools, but does not load the WASM binary
/// itself (callers are responsible for that).
#[derive(Debug, Clone)]
pub struct SkillDefinition {
    /// The name of the skill, used for identification and invocation.
    pub name: String,

    /// A human-readable description of what the skill does.
    pub description: String,

    /// The list of tool names provided by this skill.
    pub tools: Vec<String>,

    /// Optional path to the skill's compiled WASM binary.
    ///
    /// If `None`, the skill may use built-in or HTTP-based tools only.
    pub wasm_path: Option<PathBuf>,

    /// Additional metadata key-value pairs extracted from the frontmatter.
    pub metadata: HashMap<String, String>,
}

/// Internal structure used for YAML deserialization.
#[derive(Debug, serde::Deserialize)]
struct RawSkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tools: Option<serde_yaml::Value>,
    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

/// Parse a SKILL.md markdown file's YAML frontmatter.
///
/// Extracts the YAML frontmatter delimited by `---` markers and deserializes
/// it into a [`SkillDefinition`]. Only the `name` field is required; all
/// other fields default to sensible empty values.
///
/// # Format
///
/// ```yaml
/// ---
/// name: my-skill
/// description: Does something useful
/// tools:
///   - tool_a
///   - tool_b
/// ---
/// # Markdown body follows...
/// ```
///
/// # Errors
///
/// Returns [`SkillError::InvalidFormat`] if the frontmatter markers (`---`)
/// are missing or malformed. Returns [`SkillError::MissingField`] if the
/// `name` field is absent. Returns [`SkillError::ParseError`] if the YAML
/// content cannot be parsed.
pub fn parse_skill_frontmatter(markdown: &str) -> Result<SkillDefinition, SkillError> {
    let content = markdown.trim();

    // Must start with "---"
    if !content.starts_with("---") {
        return Err(SkillError::InvalidFormat(
            "SKILL.md must start with YAML frontmatter delimited by ---".into(),
        ));
    }

    // Find closing "---" after the first line
    let after_opening = content
        .strip_prefix("---")
        .unwrap()
        .trim_start()
        .trim_start_matches('\n');

    let end = after_opening
        .find("\n---")
        .ok_or_else(|| SkillError::InvalidFormat("Missing closing --- delimiter".into()))?;

    let yaml_str = &after_opening[..end];

    // Parse with serde_yaml
    let raw: RawSkillFrontmatter =
        serde_yaml::from_str(yaml_str).map_err(|e| SkillError::ParseError(e.to_string()))?;

    // Map extra metadata fields to strings
    let metadata: HashMap<String, String> = raw
        .extra
        .into_iter()
        .filter_map(|(k, v)| match v {
            serde_yaml::Value::String(s) => Some((k, s)),
            serde_yaml::Value::Number(n) => Some((k, n.to_string())),
            serde_yaml::Value::Bool(b) => Some((k, b.to_string())),
            _ => None,
        })
        .collect();

    // Parse tools from YAML value
    let tools = parse_tools_value(raw.tools);

    let name = raw.name.ok_or_else(|| {
        SkillError::MissingField("name".into())
    })?;

    Ok(SkillDefinition {
        name,
        description: raw.description.unwrap_or_default(),
        tools,
        wasm_path: None,
        metadata,
    })
}

/// Extract tool names from the parsed YAML tools value.
fn parse_tools_value(tools: Option<serde_yaml::Value>) -> Vec<String> {
    match tools {
        // YAML sequence: tools: [a, b] or tools:\n  - a\n  - b
        Some(serde_yaml::Value::Sequence(items)) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        // Empty string
        Some(serde_yaml::Value::String(s)) if s.trim().is_empty() => Vec::new(),
        // Other values: treat as empty
        Some(_) => Vec::new(),
        // Not present
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_counter_skill() {
        let markdown = "\
---
name: counter
description: A simple counter skill for testing
tools:
  - increment
  - decrement
  - get_value
---
# Counter Skill

This skill provides a simple counter.
";
        let def = parse_skill_frontmatter(markdown).expect("should parse valid frontmatter");
        assert_eq!(def.name, "counter");
        assert_eq!(def.description, "A simple counter skill for testing");
        assert_eq!(def.tools, vec!["increment", "decrement", "get_value"]);
        assert!(def.wasm_path.is_none());
        assert!(def.metadata.is_empty());
    }

    #[test]
    fn parse_missing_name_field() {
        let markdown = "\
---
description: no name here
tools:
  - echo
---
";
        let result = parse_skill_frontmatter(markdown);
        match result {
            Err(SkillError::MissingField(field)) => assert_eq!(field, "name"),
            _ => panic!("Expected MissingField error, got {:?}", result),
        }
    }

    #[test]
    fn parse_empty_tools() {
        let markdown = "\
---
name: empty-tools
description: A skill with no tools
tools: []
---
";
        let def = parse_skill_frontmatter(markdown).expect("should parse");
        assert_eq!(def.name, "empty-tools");
        assert!(def.tools.is_empty());
    }

    #[test]
    fn parse_no_yaml_frontmatter() {
        let markdown = "# Just a markdown file\nNo YAML frontmatter here.\n";
        let result = parse_skill_frontmatter(markdown);
        match result {
            Err(SkillError::InvalidFormat(_)) => {} // expected
            _ => panic!("Expected InvalidFormat error, got {:?}", result),
        }
    }

    #[test]
    fn skill_definition_struct_roundtrip() {
        let def = SkillDefinition {
            name: "test".into(),
            description: "desc".into(),
            tools: vec!["a".into(), "b".into()],
            wasm_path: Some(PathBuf::from("/tmp/skill.wasm")),
            metadata: {
                let mut m = HashMap::new();
                m.insert("key".into(), "value".into());
                m
            },
        };
        // Clone + Debug
        let cloned = def.clone();
        assert_eq!(def.name, cloned.name);
        assert_eq!(def.tools, cloned.tools);
        assert_eq!(def.wasm_path, cloned.wasm_path);
        assert_eq!(def.metadata, cloned.metadata);
        // Debug format should contain name
        let debug_str = format!("{:?}", def);
        assert!(debug_str.contains("test"));
    }
}

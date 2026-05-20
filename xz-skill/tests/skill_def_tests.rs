use std::path::PathBuf;
use xz_skill::SkillDefinition;
use xz_skill::parse_skill_frontmatter;

/// Helper to read a fixture file as a string.
fn read_fixture(relative_path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture at {:?}: {}", path, e))
}

#[test]
fn parse_valid_counter_skill() {
    let markdown = read_fixture("fixtures/skills/counter/SKILL.md");
    let def = parse_skill_frontmatter(&markdown).expect("should parse counter skill");
    assert_eq!(def.name, "counter");
    assert_eq!(
        def.description,
        "A simple counter skill for testing"
    );
    // The fixture has "get_value" not "get_count"
    assert_eq!(
        def.tools,
        vec!["increment", "decrement", "get_value"]
    );
    assert!(def.wasm_path.is_none());
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
    assert!(
        result.is_err(),
        "Expected error for missing name field, got Ok"
    );
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("name"),
        "Error should mention 'name', got: {}",
        err_str
    );
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
    assert!(def.tools.is_empty(), "tools should be empty");
}

#[test]
fn parse_no_yaml_frontmatter() {
    let markdown = "# Just a markdown file\nNo YAML frontmatter here.\n";
    let result = parse_skill_frontmatter(markdown);
    assert!(
        result.is_err(),
        "Expected error for missing frontmatter"
    );
}

#[test]
fn skill_definition_struct_roundtrip() {
    let def = SkillDefinition {
        name: "integration-test".into(),
        description: "Testing struct creation".into(),
        tools: vec!["tool_a".into(), "tool_b".into()],
        wasm_path: Some(PathBuf::from("/tmp/test.wasm")),
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert("key".into(), "value".into());
            m
        },
    };
    // Clone + Debug + field access
    let cloned = def.clone();
    assert_eq!(def.name, cloned.name);
    assert_eq!(def.tools, cloned.tools);
    let debug_str = format!("{:?}", def);
    assert!(debug_str.contains("integration-test"));
}

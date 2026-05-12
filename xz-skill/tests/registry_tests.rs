use xz_skill::{FileSkillRegistry, SkillFilter, SkillRegistry};

#[tokio::test]
async fn test_register_and_list_skills() {
    let dir = tempfile::tempdir().unwrap();
    // Create a minimal skill.yaml file
    let skill_dir = dir.path().join("test-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("skill.yaml"), r#"
id: "test-skill"
name: "Test Skill"
version: "1.0.0"
description: "A test skill"
author: "test"
enabled: true
prompt: "Test prompt"
permissions: []
tools:
  - name: echo
    description: "Echo tool"
    input_schema:
      type: object
      properties: {}
    tool_type:
      type: builtin
      handler: echo
created_at: 0
updated_at: 0
"#).unwrap();

    let registry = FileSkillRegistry::new(dir.path()).await.unwrap();
    let filter = SkillFilter::default();
    let skills = registry.list(&filter).await.unwrap();

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "Test Skill");
    assert_eq!(skills[0].tool_count, 1);
}

#[tokio::test]
async fn test_search_skills() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("weather-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("skill.yaml"), r#"
id: "weather-skill"
name: "Weather Query"
version: "1.0.0"
description: "Query weather information"
author: "org.xiaozhu"
enabled: true
prompt: "You are a weather assistant."
permissions:
  - Network
tools:
  - name: get_weather
    description: "Get weather for a city"
    input_schema:
      type: object
      properties:
        city:
          type: string
      required: [city]
    tool_type:
      type: http
      url: "https://api.weather.com/v1/current"
      method: "GET"
      headers: {}
      timeout_ms: 5000
created_at: 0
updated_at: 0
"#).unwrap();

    let registry = FileSkillRegistry::new(dir.path()).await.unwrap();

    // Search by name
    let results = registry.search("weather").await.unwrap();
    assert_eq!(results.len(), 1);

    // Search by description
    let results = registry.search("Query").await.unwrap();
    assert_eq!(results.len(), 1);

    // Search non-matching
    let results = registry.search("nonexistent").await.unwrap();
    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_enable_disable_skill() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("test-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("skill.yaml"), r#"
id: "test-skill"
name: "Test"
version: "1.0.0"
description: "Test"
author: "test"
enabled: true
prompt: "Test"
permissions: []
tools: []
created_at: 0
updated_at: 0
"#).unwrap();

    let registry = FileSkillRegistry::new(dir.path()).await.unwrap();

    // Disable
    registry.enable("test-skill", false).await.unwrap();
    let skill = registry.get("test-skill").await.unwrap().unwrap();
    assert!(!skill.enabled);

    // Re-enable
    registry.enable("test-skill", true).await.unwrap();
    let skill = registry.get("test-skill").await.unwrap().unwrap();
    assert!(skill.enabled);
}

#[tokio::test]
async fn test_unregister_skill() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("test-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("skill.yaml"), r#"
id: "test-skill"
name: "Test"
version: "1.0.0"
description: "Test"
author: "test"
enabled: true
prompt: "Test"
permissions: []
tools: []
created_at: 0
updated_at: 0
"#).unwrap();

    let registry = FileSkillRegistry::new(dir.path()).await.unwrap();

    // Verify it exists
    assert_eq!(registry.count().await.unwrap(), 1);

    // Unregister
    registry.unregister("test-skill").await.unwrap();
    assert_eq!(registry.count().await.unwrap(), 0);

    // Unregister non-existing
    let err = registry.unregister("nonexistent").await.unwrap_err();
    assert!(matches!(err, xz_skill::SkillError::NotFound(_)));
}

#[tokio::test]
async fn test_preflight_check() {
    use xz_skill::{DefaultSkillRuntime, Skill, SkillRuntime};

    let runtime = DefaultSkillRuntime::new(None);
    let skill = Skill {
        id: "test".into(),
        name: "Test".into(),
        version: "1.0.0".into(),
        description: "".into(),
        author: "".into(),
        prompt: "".into(), // Empty prompt should trigger warning
        tools: vec![],
        config_schema: None,
        default_config: None,
        permissions: vec![],
        enabled: true,
        created_at: 0,
        updated_at: 0,
        min_agent_version: None,
    };

    let warnings = runtime.preflight_check(&skill).await.unwrap();
    assert!(warnings.iter().any(|w| w.message.contains("prompt")));
}

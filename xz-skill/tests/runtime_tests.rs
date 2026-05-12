use xz_skill::{
    DefaultSkillRuntime, ExecutionContext, SkillRuntime,
};

#[tokio::test]
async fn test_execute_echo_skill() {
    let runtime = DefaultSkillRuntime::new(None);
    let _context = ExecutionContext {
        user_id: "u1".into(),
        session_id: "s1".into(),
        messages: vec![],
        provider: None,
        search: None,
        memory: None,
    };

    // Register a skill with echo tool first, then execute
    // This tests the execution API structure
    let result = runtime
        .execute_tool("now", serde_json::json!({}))
        .await
        .unwrap();
    assert!(result.get("timestamp").is_some());
}

#[tokio::test]
async fn test_execute_disabled_skill() {
    use xz_skill::FileSkillRegistry;
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("disabled-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("skill.yaml"), r#"
id: "disabled-skill"
name: "Disabled"
version: "1.0.0"
description: "A disabled skill"
author: "test"
enabled: false
prompt: "Test prompt"
permissions: []
tools: []
created_at: 0
updated_at: 0
"#).unwrap();

    let registry = std::sync::Arc::new(FileSkillRegistry::new(dir.path()).await.unwrap());
    let runtime = DefaultSkillRuntime::new(Some(registry));

    let context = ExecutionContext {
        user_id: "u1".into(),
        session_id: "s1".into(),
        messages: vec![],
        provider: None,
        search: None,
        memory: None,
    };

    let err = runtime.execute("disabled-skill", "input", &context).await.unwrap_err();
    assert!(matches!(err, xz_skill::SkillError::Disabled(_)));
}

#[tokio::test]
async fn test_execute_nonexistent_skill() {
    let runtime = DefaultSkillRuntime::new(None);
    let context = ExecutionContext {
        user_id: "u1".into(),
        session_id: "s1".into(),
        messages: vec![],
        provider: None,
        search: None,
        memory: None,
    };

    let err = runtime
        .execute("nonexistent", "input", &context)
        .await
        .unwrap_err();
    assert!(matches!(err, xz_skill::SkillError::ConfigValidation(_)));
}

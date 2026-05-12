use xz_skill::{
    DefaultSkillRuntime, FileSkillRegistry, ExecutionContext, SkillFilter,
    SkillRegistry, SkillRuntime,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Set up registry from skills directory
    let registry = FileSkillRegistry::new("./skills".as_ref()).await?;

    // 2. Register a simple skill programmatically
    let skill_yaml = r#"
id: "echo-skill"
name: "Echo Skill"
version: "1.0.0"
description: "Echoes back the input"
author: "org.xiaozhu"
enabled: true
prompt: "You are an echo assistant. Repeat the user's input back to them."
permissions: []
tools:
  - name: echo
    description: "Echo back the input"
    input_schema:
      type: object
      properties:
        input:
          type: string
      required: [input]
    tool_type:
      type: builtin
      handler: echo
min_agent_version: ">=0.1.0"
"#;

    let skill: xz_skill::Skill = serde_yaml::from_str(skill_yaml)?;
    let result = registry.register(skill).await?;
    println!("Register result: {:?}", result);

    // 3. List skills
    let filter = SkillFilter {
        enabled_only: true,
        ..Default::default()
    };
    let skills = registry.list(&filter).await?;
    for s in &skills {
        println!("  {} v{} — {} ({})", s.name, s.version, s.description, s.id);
    }

    // 4. Create runtime
    let runtime = DefaultSkillRuntime::new(Some(std::sync::Arc::new(registry)));

    // 5. Execute
    let context = ExecutionContext {
        user_id: "user-1".into(),
        session_id: "session-1".into(),
        messages: vec![],
        provider: None,
        search: None,
        memory: None,
    };

    let output = runtime.execute("echo-skill", "Hello, World!", &context).await?;
    println!("Response: {}", output.content);
    println!("Duration: {}ms", output.duration_ms);

    Ok(())
}

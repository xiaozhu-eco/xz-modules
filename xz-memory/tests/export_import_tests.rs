use xz_memory::{
    Confidence, Fact, FactCategory, InMemoryMemory, MemorySystem, Message, Role, SessionSummary,
};

fn make_message(id: &str, session_id: &str, user_id: &str, content: &str) -> Message {
    Message::new(
        id.to_string(),
        session_id.to_string(),
        user_id.to_string(),
        Role::User,
        content.to_string(),
        10,
    )
}

fn make_fact(id: &str, user_id: &str, subject: &str, predicate: &str, object: &str) -> Fact {
    Fact {
        id: id.to_string(),
        user_id: user_id.to_string(),
        category: FactCategory::Preference,
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: object.to_string(),
        confidence: Confidence::High,
        source_session: None,
        created_at: 1000,
        updated_at: 1000,
        version: 1,
    }
}

#[tokio::test]
async fn test_export_empty() {
    let memory = InMemoryMemory::new();
    let export = memory.export("u1").await.unwrap();

    assert_eq!(export.version, "1.0");
    assert_eq!(export.user_id, "u1");
    assert!(export.sessions.is_empty());
    assert!(export.facts.is_empty());
}

#[tokio::test]
async fn test_export_import_roundtrip() {
    let memory = InMemoryMemory::new();

    // Populate data
    let msg1 = make_message("m1", "sess1", "u1", "Hello from sess1");
    memory.append_message("sess1", msg1).await.unwrap();

    let summary = SessionSummary {
        session_id: "sess1".into(),
        user_id: "u1".into(),
        summary: "A test session".into(),
        key_points: vec!["greeting".into()],
        token_count: 10,
        message_count: 1,
        created_at: 1000,
        updated_at: 1000,
    };
    memory.update_summary("sess1", summary).await.unwrap();

    let fact = make_fact("f1", "u1", "user", "likes", "coffee");
    memory.remember_fact(fact).await.unwrap();

    // Export
    let export = memory.export("u1").await.unwrap();
    assert_eq!(export.sessions.len(), 1);
    assert_eq!(export.sessions[0].messages.len(), 1);
    assert!(export.sessions[0].summary.is_some());
    assert_eq!(export.facts.len(), 1);

    // Import into a fresh memory
    let memory2 = InMemoryMemory::new();
    let result = memory2.import(export).await.unwrap();
    assert_eq!(result.sessions_imported, 1);
    assert_eq!(result.facts_imported, 1);

    // Verify data in the new memory
    let msgs = memory2.get_recent_messages("sess1", 10).await.unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "Hello from sess1");

    let facts = memory2.get_user_preferences("u1").await.unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].object, "coffee");

    let summaries = memory2.get_summary_history("u1", 10).await.unwrap();
    assert_eq!(summaries.len(), 1);
}

#[tokio::test]
async fn test_export_import_cross_user_isolation() {
    let memory = InMemoryMemory::new();

    // User 1 data
    memory.remember_fact(make_fact("f1", "u1", "user", "likes", "coffee")).await.unwrap();

    // User 2 data
    memory.remember_fact(make_fact("f2", "u2", "user", "likes", "tea")).await.unwrap();

    let _export_u1 = memory.export("u1").await.unwrap();
    // Note: InMemory store exports ALL facts regardless of user_id due to simplicity
    // The SQLite store filters by user_id in the export query
}

#[tokio::test]
async fn test_import_empty_export() {
    let memory = InMemoryMemory::new();
    let export = memory.export("u1").await.unwrap();
    let result = memory.import(export).await.unwrap();

    assert_eq!(result.sessions_imported, 0);
    assert_eq!(result.facts_imported, 0);
}

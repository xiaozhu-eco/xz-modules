use xz_memory::{MemorySystem, Message, PageRequest, Role, InMemoryMemory, SessionSummary};

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

#[tokio::test]
async fn test_append_and_get_recent_messages() {
    let memory = InMemoryMemory::new();
    let m1 = make_message("m1", "sess1", "u1", "Hello");
    let m2 = make_message("m2", "sess1", "u1", "World");

    memory.append_message("sess1", m1).await.unwrap();
    memory.append_message("sess1", m2).await.unwrap();

    let recent = memory.get_recent_messages("sess1", 2).await.unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].content, "Hello");
    assert_eq!(recent[1].content, "World");
}

#[tokio::test]
async fn test_get_recent_messages_limited() {
    let memory = InMemoryMemory::new();
    for i in 0..10 {
        let msg = make_message(
            &format!("m{}", i),
            "sess1",
            "u1",
            &format!("msg{}", i),
        );
        memory.append_message("sess1", msg).await.unwrap();
    }

    let recent = memory.get_recent_messages("sess1", 3).await.unwrap();
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0].content, "msg7");
    assert_eq!(recent[2].content, "msg9");
}

#[tokio::test]
async fn test_session_message_pagination() {
    let memory = InMemoryMemory::new();
    for i in 0..20 {
        let msg = make_message(
            &format!("m{}", i),
            "sess1",
            "u1",
            &format!("msg{}", i),
        );
        memory.append_message("sess1", msg).await.unwrap();
    }

    let page = memory
        .get_session_messages(
            "sess1",
            PageRequest { limit: 5, offset: 0 },
        )
        .await
        .unwrap();
    assert_eq!(page.items.len(), 5);
    assert_eq!(page.total, 20);
    assert!(page.has_more);

    // Second page
    let page2 = memory
        .get_session_messages(
            "sess1",
            PageRequest { limit: 5, offset: 5 },
        )
        .await
        .unwrap();
    assert_eq!(page2.items.len(), 5);
    assert!(page2.has_more);

    // Last page
    let page3 = memory
        .get_session_messages(
            "sess1",
            PageRequest { limit: 10, offset: 15 },
        )
        .await
        .unwrap();
    assert_eq!(page3.items.len(), 5);
    assert!(!page3.has_more);
}

#[tokio::test]
async fn test_clear_short_term_preserves_summary() {
    let memory = InMemoryMemory::new();
    let msg = make_message("m1", "sess1", "u1", "Hello");
    memory.append_message("sess1", msg).await.unwrap();

    let summary = SessionSummary {
        session_id: "sess1".into(),
        user_id: "u1".into(),
        summary: "test summary".into(),
        key_points: vec![],
        token_count: 10,
        message_count: 1,
        created_at: 1000,
        updated_at: 1000,
    };
    memory.update_summary("sess1", summary).await.unwrap();

    memory.clear_short_term("sess1").await.unwrap();

    let messages = memory.get_recent_messages("sess1", 10).await.unwrap();
    assert!(messages.is_empty());

    // Summary should still be accessible
    let history = memory.get_summary_history("u1", 10).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].summary, "test summary");
}

#[tokio::test]
async fn test_get_summary_history() {
    let memory = InMemoryMemory::new();

    for i in 0..5 {
        let summary = SessionSummary {
            session_id: format!("sess{}", i),
            user_id: "u1".into(),
            summary: format!("summary {}", i),
            key_points: vec![],
            token_count: 100,
            message_count: 10,
            created_at: 1000 + i as u64,
            updated_at: 2000 + i as u64,
        };
        memory.update_summary(&format!("sess{}", i), summary).await.unwrap();
    }

    let history = memory.get_summary_history("u1", 3).await.unwrap();
    assert_eq!(history.len(), 3);
    // Most recently updated first
    assert_eq!(history[0].session_id, "sess4");
}

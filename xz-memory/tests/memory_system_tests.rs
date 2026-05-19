//! Comprehensive tests for the MemorySystem trait using InMemoryMemory backend.

use std::time::SystemTime;

use xz_memory::{
    CompactionStrategy, Confidence, Fact, FactCategory, FactRecallOptions, InMemoryMemory,
    MemorySystem, Message, PageRequest, Role, SessionSummary, UpsertResult,
};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

fn make_message(
    session_id: &str,
    user_id: &str,
    content: &str,
    role: Role,
    seq_offset: u64,
) -> Message {
    Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        user_id: user_id.to_string(),
        role,
        content: content.to_string(),
        token_count: content.len(),
        created_at: now_ms() - (1000 * seq_offset),
        seq: seq_offset,
    }
}

// ============================================================================
// Short-term Memory Tests
// ============================================================================

#[tokio::test]
async fn test_append_and_get_recent_messages() {
    let mem = InMemoryMemory::new();
    let session_id = "test-session-1";
    let user_id = "user-1";

    mem.append_message(session_id, make_message(session_id, user_id, "Hello", Role::User, 0))
        .await
        .unwrap();
    mem.append_message(
        session_id,
        make_message(session_id, user_id, "Hi there!", Role::Assistant, 1),
    )
    .await
    .unwrap();
    mem.append_message(
        session_id,
        make_message(session_id, user_id, "How are you?", Role::User, 2),
    )
    .await
    .unwrap();

    let recent = mem.get_recent_messages(session_id, 2).await.unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].content, "Hi there!");
    assert_eq!(recent[1].content, "How are you?");

    let all = mem.get_recent_messages(session_id, 100).await.unwrap();
    assert_eq!(all.len(), 3);

    let empty = mem.get_recent_messages("nonexistent", 10).await.unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn test_get_session_messages_pagination() {
    let mem = InMemoryMemory::new();
    let session_id = "test-session-paginate";

    for i in 0..25 {
        mem.append_message(
            session_id,
            make_message(session_id, "user-1", &format!("msg-{}", i), Role::User, i),
        )
        .await
        .unwrap();
    }

    let page1 =
        mem.get_session_messages(session_id, PageRequest { limit: 10, offset: 0 }).await.unwrap();
    assert_eq!(page1.items.len(), 10);
    assert_eq!(page1.total, 25);
    assert!(page1.has_more);

    let page2 =
        mem.get_session_messages(session_id, PageRequest { limit: 10, offset: 10 }).await.unwrap();
    assert_eq!(page2.items.len(), 10);
    assert!(page2.has_more);

    let page3 =
        mem.get_session_messages(session_id, PageRequest { limit: 10, offset: 20 }).await.unwrap();
    assert_eq!(page3.items.len(), 5);
    assert!(!page3.has_more);

    let empty_page =
        mem.get_session_messages("no-such-session", PageRequest::default()).await.unwrap();
    assert!(empty_page.items.is_empty());
    assert_eq!(empty_page.total, 0);
}

#[tokio::test]
async fn test_clear_short_term() {
    let mem = InMemoryMemory::new();
    let session_id = "test-clear";

    mem.append_message(session_id, make_message(session_id, "user-1", "msg1", Role::User, 0))
        .await
        .unwrap();
    mem.append_message(session_id, make_message(session_id, "user-1", "msg2", Role::Assistant, 1))
        .await
        .unwrap();

    assert_eq!(mem.get_recent_messages(session_id, 10).await.unwrap().len(), 2);

    mem.clear_short_term(session_id).await.unwrap();
    assert!(mem.get_recent_messages(session_id, 10).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_evict_oldest_messages() {
    let mem = InMemoryMemory::new();
    let session_id = "test-evict";

    for i in 0..10 {
        mem.append_message(
            session_id,
            make_message(session_id, "user-1", &format!("msg-{}", i), Role::User, i),
        )
        .await
        .unwrap();
    }

    let evicted = mem.evict_oldest_messages(session_id, 5).await.unwrap();
    assert_eq!(evicted, 5);

    let remaining = mem.get_recent_messages(session_id, 20).await.unwrap();
    assert_eq!(remaining.len(), 5);
    assert_eq!(remaining[0].content, "msg-5");

    let evicted2 = mem.evict_oldest_messages(session_id, 100).await.unwrap();
    assert_eq!(evicted2, 0);
    assert_eq!(mem.get_recent_messages(session_id, 100).await.unwrap().len(), 5);
}

// ============================================================================
// Summary Memory Tests
// ============================================================================

#[tokio::test]
async fn test_update_and_get_summary_history() {
    let mem = InMemoryMemory::new();
    let user_id = "user-summary";

    let summary1 = SessionSummary {
        session_id: "session-a".to_string(),
        user_id: user_id.to_string(),
        summary: "Summary of session A".to_string(),
        key_points: vec!["point 1".into(), "point 2".into()],
        token_count: 100,
        message_count: 5,
        created_at: now_ms(),
        updated_at: now_ms(),
    };

    let summary2 = SessionSummary {
        session_id: "session-b".to_string(),
        user_id: user_id.to_string(),
        summary: "Summary of session B".to_string(),
        key_points: vec!["point 3".into()],
        token_count: 200,
        message_count: 10,
        created_at: now_ms(),
        updated_at: now_ms() + 1,
    };

    let summary3 = SessionSummary {
        session_id: "session-c".to_string(),
        user_id: "other-user".to_string(),
        summary: "Other user summary".to_string(),
        key_points: vec![],
        token_count: 50,
        message_count: 2,
        created_at: now_ms(),
        updated_at: now_ms(),
    };

    mem.update_summary("session-a", summary1.clone()).await.unwrap();
    mem.update_summary("session-b", summary2.clone()).await.unwrap();
    mem.update_summary("session-c", summary3).await.unwrap();

    let history = mem.get_summary_history(user_id, 10).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].session_id, "session-b");
    assert_eq!(history[1].session_id, "session-a");

    let limited = mem.get_summary_history(user_id, 1).await.unwrap();
    assert_eq!(limited.len(), 1);
}

// ============================================================================
// Fact Memory Tests
// ============================================================================

#[tokio::test]
async fn test_remember_and_recall_facts() {
    let mem = InMemoryMemory::new();
    let user_id = "user-facts";

    let fact1 = Fact {
        id: "fact-1".to_string(),
        user_id: user_id.to_string(),
        category: FactCategory::Preference,
        subject: "user".to_string(),
        predicate: "likes".to_string(),
        object: "coffee".to_string(),
        confidence: Confidence::High,
        source_session: Some("session-1".into()),
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    };

    let fact2 = Fact {
        id: "fact-2".to_string(),
        user_id: user_id.to_string(),
        category: FactCategory::PersonalInfo,
        subject: "user".to_string(),
        predicate: "lives_in".to_string(),
        object: "Beijing".to_string(),
        confidence: Confidence::Medium,
        source_session: Some("session-1".into()),
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    };

    let result1 = mem.remember_fact(fact1.clone()).await.unwrap();
    assert!(matches!(result1, UpsertResult::Created));

    let result2 = mem.remember_fact(fact2.clone()).await.unwrap();
    assert!(matches!(result2, UpsertResult::Created));

    let result3 = mem.remember_fact(fact1.clone()).await.unwrap();
    assert!(matches!(result3, UpsertResult::Unchanged));

    let mut fact1_updated = fact1.clone();
    fact1_updated.object = "tea".to_string();
    let result4 = mem.remember_fact(fact1_updated).await.unwrap();
    assert!(matches!(result4, UpsertResult::Updated { .. }));

    let options = FactRecallOptions::default();
    let page = mem.recall_facts(user_id, "coffee", &options).await.unwrap();
    assert_eq!(page.items.len(), 0); // Updated to "tea"

    let page = mem.recall_facts(user_id, "Beijing", &options).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].predicate, "lives_in");
}

#[tokio::test]
async fn test_recall_facts_with_filters() {
    let mem = InMemoryMemory::new();
    let user_id = "user-filters";

    for i in 0..15 {
        let fact = Fact {
            id: format!("fact-{}", i),
            user_id: user_id.to_string(),
            category: if i % 2 == 0 {
                FactCategory::Preference
            } else {
                FactCategory::PersonalInfo
            },
            subject: format!("thing-{}", i),
            predicate: "has_property".to_string(),
            object: format!("value-{}", i),
            confidence: if i < 5 { Confidence::High } else { Confidence::Low },
            source_session: None,
            created_at: now_ms(),
            updated_at: now_ms() + i as u64,
            version: 1,
        };
        mem.remember_fact(fact).await.unwrap();
    }

    let options = FactRecallOptions {
        categories: Some(vec![FactCategory::Preference]),
        ..Default::default()
    };
    let page = mem.recall_facts(user_id, "property", &options).await.unwrap();
    assert!(page.items.iter().all(|f| f.category == FactCategory::Preference));

    let options =
        FactRecallOptions { min_confidence: Some(Confidence::Medium), ..Default::default() };
    let page = mem.recall_facts(user_id, "property", &options).await.unwrap();
    assert!(page.items.iter().all(|f| f.confidence >= Confidence::Medium));

    let options =
        FactRecallOptions { page: PageRequest { limit: 5, offset: 0 }, ..Default::default() };
    let page = mem.recall_facts(user_id, "property", &options).await.unwrap();
    assert_eq!(page.items.len(), 5);
    assert!(page.has_more);
}

#[tokio::test]
async fn test_get_user_preferences() {
    let mem = InMemoryMemory::new();
    let user_id = "user-prefs";

    mem.remember_fact(Fact {
        id: "pref-1".into(),
        user_id: user_id.into(),
        category: FactCategory::Preference,
        subject: "food".into(),
        predicate: "prefers".into(),
        object: "sushi".into(),
        confidence: Confidence::High,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    mem.remember_fact(Fact {
        id: "pref-2".into(),
        user_id: user_id.into(),
        category: FactCategory::Preference,
        subject: "color".into(),
        predicate: "prefers".into(),
        object: "blue".into(),
        confidence: Confidence::Medium,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    mem.remember_fact(Fact {
        id: "info-1".into(),
        user_id: user_id.into(),
        category: FactCategory::PersonalInfo,
        subject: "user".into(),
        predicate: "age".into(),
        object: "30".into(),
        confidence: Confidence::High,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    let prefs = mem.get_user_preferences(user_id).await.unwrap();
    assert_eq!(prefs.len(), 2);
    assert!(prefs.iter().all(|f| f.category == FactCategory::Preference));
}

#[tokio::test]
async fn test_delete_fact() {
    let mem = InMemoryMemory::new();

    mem.remember_fact(Fact {
        id: "fact-del".into(),
        user_id: "user-1".into(),
        category: FactCategory::PersonalInfo,
        subject: "test".into(),
        predicate: "is".into(),
        object: "present".into(),
        confidence: Confidence::Medium,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    mem.delete_fact("fact-del").await.unwrap();

    let result = mem.delete_fact("fact-del").await;
    assert!(result.is_err());

    let result = mem.delete_fact("no-such-fact").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_compact_facts_merge_similar() {
    let mem = InMemoryMemory::new();
    let user_id = "user-compact";

    mem.remember_fact(Fact {
        id: "merge-1".into(),
        user_id: user_id.into(),
        category: FactCategory::Preference,
        subject: "food".into(),
        predicate: "likes".into(),
        object: "pizza".into(),
        confidence: Confidence::Low,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    mem.remember_fact(Fact {
        id: "merge-2".into(),
        user_id: user_id.into(),
        category: FactCategory::Preference,
        subject: "food".into(),
        predicate: "likes".into(),
        object: "pizza".into(),
        confidence: Confidence::High,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms() + 1,
        version: 1,
    })
    .await
    .unwrap();

    mem.remember_fact(Fact {
        id: "keep-1".into(),
        user_id: user_id.into(),
        category: FactCategory::PersonalInfo,
        subject: "name".into(),
        predicate: "is".into(),
        object: "Alice".into(),
        confidence: Confidence::High,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    let result = mem.compact_facts(user_id, CompactionStrategy::MergeSimilar).await.unwrap();
    assert_eq!(result.facts_merged, 0);
    assert_eq!(result.facts_kept, 2); // The High confidence one + "keep-1"
}

#[tokio::test]
async fn test_compact_facts_remove_low_confidence() {
    let mem = InMemoryMemory::new();
    let user_id = "user-compact-low";

    mem.remember_fact(Fact {
        id: "low-1".into(),
        user_id: user_id.into(),
        category: FactCategory::PersonalInfo,
        subject: "a".into(),
        predicate: "is".into(),
        object: "b".into(),
        confidence: Confidence::Speculative,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    mem.remember_fact(Fact {
        id: "high-1".into(),
        user_id: user_id.into(),
        category: FactCategory::PersonalInfo,
        subject: "c".into(),
        predicate: "is".into(),
        object: "d".into(),
        confidence: Confidence::High,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    let result =
        mem.compact_facts(user_id, CompactionStrategy::RemoveLowConfidence(0.5)).await.unwrap();
    assert_eq!(result.facts_removed, 1);
    assert_eq!(result.facts_kept, 1);
}

// ============================================================================
// Export / Import Tests
// ============================================================================

#[tokio::test]
async fn test_export_import_roundtrip() {
    let mem = InMemoryMemory::new();
    let user_id = "user-roundtrip";
    let session_id = "session-roundtrip";

    for i in 0..5 {
        mem.append_message(
            session_id,
            make_message(session_id, user_id, &format!("msg-{}", i), Role::User, i),
        )
        .await
        .unwrap();
    }

    mem.update_summary(
        session_id,
        SessionSummary {
            session_id: session_id.to_string(),
            user_id: user_id.to_string(),
            summary: "Roundtrip test".to_string(),
            key_points: vec!["p1".into()],
            token_count: 50,
            message_count: 5,
            created_at: now_ms(),
            updated_at: now_ms(),
        },
    )
    .await
    .unwrap();

    mem.remember_fact(Fact {
        id: "rt-fact-1".into(),
        user_id: user_id.into(),
        category: FactCategory::Preference,
        subject: "test".into(),
        predicate: "works".into(),
        object: "yes".into(),
        confidence: Confidence::Confirmed,
        source_session: Some(session_id.into()),
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    })
    .await
    .unwrap();

    // Export
    let exported = mem.export(user_id).await.unwrap();
    assert_eq!(exported.user_id, user_id);
    assert_eq!(exported.sessions.len(), 1);
    assert_eq!(exported.sessions[0].messages.len(), 5);
    assert!(exported.sessions[0].summary.is_some());
    assert_eq!(exported.facts.len(), 1);

    let mem2 = InMemoryMemory::new();
    let result = mem2.import(exported).await.unwrap();
    assert_eq!(result.sessions_imported, 1);
    assert_eq!(result.facts_imported, 1);

    let msgs = mem2.get_recent_messages(session_id, 10).await.unwrap();
    assert_eq!(msgs.len(), 5);
    let facts = mem2.recall_facts(user_id, "works", &FactRecallOptions::default()).await.unwrap();
    assert_eq!(facts.items.len(), 1);
}

// ============================================================================
// Stats Tests
// ============================================================================

#[tokio::test]
async fn test_stats() {
    let mem = InMemoryMemory::new();
    let user_id = "user-stats";

    for i in 0..3 {
        let session_id = format!("session-stats-{}", i);
        mem.append_message(
            &session_id,
            make_message(&session_id, user_id, &format!("msg-{}", i), Role::User, 0),
        )
        .await
        .unwrap();
    }

    for i in 0..5 {
        mem.remember_fact(Fact {
            id: format!("stat-fact-{}", i),
            user_id: user_id.to_string(),
            category: FactCategory::PersonalInfo,
            subject: format!("s-{}", i),
            predicate: "is".into(),
            object: format!("o-{}", i),
            confidence: Confidence::Medium,
            source_session: None,
            created_at: now_ms(),
            updated_at: now_ms(),
            version: 1,
        })
        .await
        .unwrap();
    }

    let stats = mem.stats(user_id).await.unwrap();
    assert_eq!(stats.total_sessions, 3);
    assert_eq!(stats.total_messages, 3);
    assert_eq!(stats.total_facts, 5);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_sessions_isolation() {
    let mem = InMemoryMemory::new();
    let user_a = "user-a";
    let user_b = "user-b";

    mem.append_message("sess-a", make_message("sess-a", user_a, "msg-a", Role::User, 0))
        .await
        .unwrap();
    mem.append_message("sess-b", make_message("sess-b", user_b, "msg-b", Role::User, 0))
        .await
        .unwrap();

    let a = mem.get_recent_messages("sess-a", 10).await.unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].content, "msg-a");

    let b = mem.get_recent_messages("sess-b", 10).await.unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].content, "msg-b");

    mem.clear_short_term("sess-a").await.unwrap();
    assert!(mem.get_recent_messages("sess-a", 10).await.unwrap().is_empty());
    assert_eq!(mem.get_recent_messages("sess-b", 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_empty_operations_dont_error() {
    let mem = InMemoryMemory::new();

    let msgs = mem.get_recent_messages("nonexistent", 10).await.unwrap();
    assert!(msgs.is_empty());

    let page = mem.get_session_messages("nonexistent", PageRequest::default()).await.unwrap();
    assert!(page.items.is_empty());

    let facts = mem.recall_facts("no-user", "query", &FactRecallOptions::default()).await.unwrap();
    assert!(facts.items.is_empty());

    let prefs = mem.get_user_preferences("no-user").await.unwrap();
    assert!(prefs.is_empty());

    let stats = mem.stats("no-user").await.unwrap();
    assert_eq!(stats.total_messages, 0);
    assert_eq!(stats.total_facts, 0);

    let exported = mem.export("no-user").await.unwrap();
    assert!(exported.sessions.is_empty());
}

#[tokio::test]
async fn test_fact_upsert_same_id_different_user() {
    let mem = InMemoryMemory::new();

    let fact = Fact {
        id: "same-id".into(),
        user_id: "user-1".into(),
        category: FactCategory::Preference,
        subject: "color".into(),
        predicate: "likes".into(),
        object: "red".into(),
        confidence: Confidence::High,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    };
    mem.remember_fact(fact.clone()).await.unwrap();

    let fact2 = Fact {
        id: "same-id-2".into(),
        user_id: "user-2".into(),
        category: FactCategory::Preference,
        subject: "color".into(),
        predicate: "likes".into(),
        object: "blue".into(),
        confidence: Confidence::High,
        source_session: None,
        created_at: now_ms(),
        updated_at: now_ms(),
        version: 1,
    };
    mem.remember_fact(fact2.clone()).await.unwrap();

    let user1_facts =
        mem.recall_facts("user-1", "color", &FactRecallOptions::default()).await.unwrap();
    assert_eq!(user1_facts.items.len(), 1);
    assert_eq!(user1_facts.items[0].object, "red");

    let user2_facts =
        mem.recall_facts("user-2", "color", &FactRecallOptions::default()).await.unwrap();
    assert_eq!(user2_facts.items.len(), 1);
    assert_eq!(user2_facts.items[0].object, "blue");
}

use xz_memory::{
    Confidence, Fact, FactCategory, FactRecallOptions, InMemoryMemory, MemorySystem, PageRequest,
    UpsertResult,
};

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
async fn test_remember_and_recall_fact() {
    let memory = InMemoryMemory::new();
    let fact = make_fact("f1", "u1", "user", "likes", "coffee");
    let result = memory.remember_fact(fact).await.unwrap();
    assert!(matches!(result, UpsertResult::Created));

    let recalled =
        memory.recall_facts("u1", "coffee", &FactRecallOptions::default()).await.unwrap();
    assert_eq!(recalled.total, 1);
    assert_eq!(recalled.items[0].object, "coffee");
}

#[tokio::test]
async fn test_fact_upsert_dedup() {
    let memory = InMemoryMemory::new();

    let f1 = make_fact("f1", "u1", "user", "likes", "coffee");
    let result = memory.remember_fact(f1).await.unwrap();
    assert!(matches!(result, UpsertResult::Created));

    // Same (user_id, subject, predicate) — should update
    let f2 = Fact {
        object: "espresso".to_string(),
        ..make_fact("f2", "u1", "user", "likes", "espresso")
    };
    let result = memory.remember_fact(f2).await.unwrap();
    assert!(matches!(result, UpsertResult::Updated { .. }));

    // Only one fact should exist for the same subject+predicate
    let recalled =
        memory.recall_facts("u1", "espresso", &FactRecallOptions::default()).await.unwrap();
    assert_eq!(recalled.total, 1);
    assert_eq!(recalled.items[0].object, "espresso");
}

#[tokio::test]
async fn test_delete_fact() {
    let memory = InMemoryMemory::new();
    let fact = make_fact("f1", "u1", "user", "likes", "coffee");
    memory.remember_fact(fact).await.unwrap();

    memory.delete_fact("f1").await.unwrap();

    let recalled =
        memory.recall_facts("u1", "coffee", &FactRecallOptions::default()).await.unwrap();
    assert_eq!(recalled.total, 0);
}

#[tokio::test]
async fn test_delete_nonexistent_fact() {
    let memory = InMemoryMemory::new();
    let err = memory.delete_fact("nonexistent").await.unwrap_err();
    assert!(matches!(err, xz_memory::MemoryError::FactNotFound(_)));
}

#[tokio::test]
async fn test_get_user_preferences() {
    let memory = InMemoryMemory::new();
    let f1 = make_fact("f1", "u1", "user", "likes", "coffee");
    let f2 = Fact {
        category: FactCategory::PersonalInfo,
        ..make_fact("f2", "u1", "user", "from", "Beijing")
    };
    memory.remember_fact(f1).await.unwrap();
    memory.remember_fact(f2).await.unwrap();

    let prefs = memory.get_user_preferences("u1").await.unwrap();
    assert_eq!(prefs.len(), 1);
    assert_eq!(prefs[0].category, FactCategory::Preference);
}

#[tokio::test]
async fn test_fact_pagination() {
    let memory = InMemoryMemory::new();
    for i in 0..10 {
        // Each fact must have a different (subject, predicate) to avoid upsert dedup
        let fact = Fact {
            id: format!("f{}", i),
            ..make_fact(
                &format!("f{}", i),
                "u1",
                &format!("item{}", i),
                "tagged",
                &format!("value{}", i),
            )
        };
        memory.remember_fact(fact).await.unwrap();
    }

    let results = memory
        .recall_facts(
            "u1",
            "item",
            &FactRecallOptions { page: PageRequest { limit: 5, offset: 0 }, ..Default::default() },
        )
        .await
        .unwrap();
    assert_eq!(results.items.len(), 5);
    assert_eq!(results.total, 10);
    assert!(results.has_more);
}

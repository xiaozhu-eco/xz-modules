use xz_memory::{
    CompactionStrategy, Confidence, Fact, FactCategory, InMemoryMemory, MemorySystem,
};

fn make_fact(id: &str, user_id: &str, subject: &str, predicate: &str, object: &str, confidence: Confidence, updated_at: u64) -> Fact {
    Fact {
        id: id.to_string(),
        user_id: user_id.to_string(),
        category: FactCategory::Preference,
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: object.to_string(),
        confidence,
        source_session: None,
        created_at: 1000,
        updated_at,
        version: 1,
    }
}

#[tokio::test]
async fn test_compact_merge_similar_with_different_objects() {
    let memory = InMemoryMemory::new();

    // Two facts with SAME (user_id, subject, predicate) but different objects.
    // The upsert logic will update the first with the second, keeping only 1 fact.
    // This test validates the compaction strategy reports correctly on the result.
    let f1 = make_fact("f1", "u1", "user", "likes", "coffee", Confidence::Low, 1000);
    let f2 = make_fact("f2", "u1", "user", "likes", "tea", Confidence::High, 1000);

    memory.remember_fact(f1).await.unwrap();
    memory.remember_fact(f2).await.unwrap();

    // After upsert, only f2 remains (it replaced f1 because same subject+predicate)
    let result = memory
        .compact_facts("u1", CompactionStrategy::MergeSimilar)
        .await
        .unwrap();

    // No duplicates to merge, since upsert already deduplicated
    assert_eq!(result.facts_merged, 0);
    assert_eq!(result.facts_kept, 1);
}

#[tokio::test]
async fn test_compact_remove_low_confidence() {
    let memory = InMemoryMemory::new();

    // Facts with DIFFERENT subject+predicate so upsert doesn't interfere
    let f1 = make_fact("f1", "u1", "user", "likes", "coffee", Confidence::Low, 1000);
    let f2 = make_fact("f2", "u1", "user", "dislikes", "tea", Confidence::High, 1000);
    let f3 = make_fact("f3", "u1", "user", "lives_in", "Beijing", Confidence::Medium, 1000);

    memory.remember_fact(f1).await.unwrap();
    memory.remember_fact(f2).await.unwrap();
    memory.remember_fact(f3).await.unwrap();

    // Remove facts with confidence below 0.5 (Low=0.35 is removed, Medium=0.6 stays, High=0.85 stays)
    let result = memory
        .compact_facts("u1", CompactionStrategy::RemoveLowConfidence(0.5))
        .await
        .unwrap();

    assert_eq!(result.facts_removed, 1); // f1 (Low) removed
    assert_eq!(result.facts_kept, 2);    // f2 (High) and f3 (Medium) kept
}

#[tokio::test]
async fn test_compact_remove_old() {
    let memory = InMemoryMemory::new();

    // Facts with different subjects/predicates
    let f1 = make_fact("f1", "u1", "user", "likes", "coffee", Confidence::Low, 100);  // old + low confidence
    let f2 = make_fact("f2", "u1", "user", "dislikes", "tea", Confidence::Low, 100);  // old + low confidence
    let f3 = make_fact("f3", "u1", "user", "lives_in", "Beijing", Confidence::High, 2000); // new + high confidence

    memory.remember_fact(f1).await.unwrap();
    memory.remember_fact(f2).await.unwrap();
    memory.remember_fact(f3).await.unwrap();

    // Remove facts updated before ts=500 with confidence < 0.6
    let result = memory
        .compact_facts("u1", CompactionStrategy::RemoveOld(500))
        .await
        .unwrap();

    // f1 and f2 have updated_at=100 (<500) and low confidence (<0.6), should be removed
    // f3 has updated_at=2000 (>500), should be kept
    assert_eq!(result.facts_removed, 2);
    assert_eq!(result.facts_kept, 1);
}

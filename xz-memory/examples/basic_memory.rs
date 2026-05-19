//! Basic memory CRUD operations.
//!
//! ```bash
//! cargo run --example basic_memory
//! ```

use xz_memory::{
    Confidence, Fact, FactCategory, FactRecallOptions, InMemoryMemory, MemoryError, MemorySystem,
    Message, Role,
};

#[tokio::main]
async fn main() -> Result<(), MemoryError> {
    let memory = InMemoryMemory::new();

    // Append messages
    let msg = Message::new(
        uuid::Uuid::new_v4().to_string(),
        "sess_1".into(),
        "user_1".into(),
        Role::User,
        "I like sci-fi novels".into(),
        5,
    );
    memory.append_message("sess_1", msg).await?;

    let msg = Message::new(
        uuid::Uuid::new_v4().to_string(),
        "sess_1".into(),
        "user_1".into(),
        Role::Assistant,
        "That's great! What authors do you like?".into(),
        8,
    );
    memory.append_message("sess_1", msg).await?;

    // Remember facts
    let fact = Fact {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: "user_1".into(),
        category: FactCategory::Preference,
        subject: "user".into(),
        predicate: "likes".into(),
        object: "sci-fi novels".into(),
        confidence: Confidence::High,
        source_session: Some("sess_1".into()),
        created_at: 1000,
        updated_at: 1000,
        version: 1,
    };
    let result = memory.remember_fact(fact).await?;
    println!("Fact upsert result: {:?}", result);

    // Search facts
    let results = memory.recall_facts("user_1", "sci-fi", &FactRecallOptions::default()).await?;
    println!("Found {} facts about sci-fi", results.total);

    // Get recent messages
    let recent = memory.get_recent_messages("sess_1", 10).await?;
    println!("Recent messages: {}", recent.len());

    // Get user preferences
    let prefs = memory.get_user_preferences("user_1").await?;
    println!("User preferences: {}", prefs.len());

    // Stats
    let stats = memory.stats("user_1").await?;
    println!(
        "Stats: {} sessions, {} messages, {} facts",
        stats.total_sessions, stats.total_messages, stats.total_facts
    );

    Ok(())
}

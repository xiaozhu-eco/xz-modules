//! Session summary generation using LLM (requires `summary` feature).
//!
//! ```bash
//! cargo run --example session_summary --features summary
//! ```

use xz_memory::{InMemoryMemory, MemorySystem, Message, Role, SessionSummary};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let memory = InMemoryMemory::new();

    // Simulate a conversation
    let messages = vec![
        ("I like sci-fi novels, especially ones about AI.", "user"),
        ("That's interesting! Have you read any Isaac Asimov?", "assistant"),
        ("Yes, I love the Foundation series. The concept of psychohistory is fascinating.", "user"),
        ("What about more modern authors like Ted Chiang?", "assistant"),
        ("Ted Chiang is brilliant. 'Exhalation' is one of my favorite collections.", "user"),
    ];

    for (_i, (content, role)) in messages.iter().enumerate() {
        let msg = Message::new(
            uuid::Uuid::new_v4().to_string(),
            "sess_1".into(),
            "user_1".into(),
            if *role == "user" { Role::User } else { Role::Assistant },
            content.to_string(),
            content.len() / 4,
        );
        memory.append_message("sess_1", msg).await?;
    }

    // With the summary feature, we'd generate a summary here
    // For this example, we manually add a summary
    let summary = SessionSummary {
        session_id: "sess_1".into(),
        user_id: "user_1".into(),
        summary: "The user enjoys sci-fi novels, particularly about AI themes. \
                  Favorite authors include Isaac Asimov (Foundation series) and Ted Chiang (Exhalation)."
            .into(),
        key_points: vec![
            "Likes sci-fi novels about AI".into(),
            "Enjoys Isaac Asimov's Foundation series".into(),
            "Appreciates Ted Chiang's Exhalation".into(),
        ],
        token_count: 100,
        message_count: 5,
        created_at: 1000,
        updated_at: 1000,
    };
    memory.update_summary("sess_1", summary.clone()).await?;

    println!("Session summary: {}", summary.summary);
    println!("Key points:");
    for (i, point) in summary.key_points.iter().enumerate() {
        println!("  {}. {}", i + 1, point);
    }

    let history = memory.get_summary_history("user_1", 10).await?;
    println!("\nSummary history entries: {}", history.len());

    Ok(())
}

//! Vector search example (requires `vector-memory` feature).
//!
//! ```bash
//! cargo run --example vector_search --features vector-memory
//! ```

#[cfg(feature = "vector-memory")]
use xz_memory::{InMemoryMemory, MemorySystem};

#[cfg(feature = "vector-memory")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let memory = InMemoryMemory::new();

    // Create a vector entry
    let vector = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    let entry = xz_embed::types::VectorEntry {
        id: uuid::Uuid::new_v4().to_string(),
        vector: vector.clone(),
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert("user_id".into(), "user_1".into());
            m.insert("type".into(), "fact_embedding".into());
            m
        },
        content: Some("user likes sci-fi novels".into()),
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        expires_at: None,
        channel: Some("memory".into()),
    };

    memory.store_vector(entry).await?;
    println!("Vector stored successfully");

    // Search similar vectors
    let query = vec![0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85];
    let results = memory.search_vector(&query, 5, 0.5).await?;
    println!("Vector search returned {} results", results.len());

    for r in &results {
        println!("  id={}, score={:.4}", r.id, r.score);
    }

    Ok(())
}

#[cfg(not(feature = "vector-memory"))]
fn main() {
    eprintln!("This example requires the 'vector-memory' feature.");
    eprintln!("Run: cargo run --example vector_search --features vector-memory");
}

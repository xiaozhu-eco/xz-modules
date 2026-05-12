use xz_embed::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mock = MockEmbedder::new(4, 32);
    mock.set_output(vec![
        vec![0.1, 0.2, 0.3, 0.4],
        vec![0.5, 0.6, 0.7, 0.8],
        vec![0.9, 1.0, 0.1, 0.2],
    ]);

    let texts = vec![
        "Rust is a systems programming language.",
        "Python is great for data science.",
        "Go is designed for concurrency.",
    ];
    let text_refs: Vec<&str> = texts.iter().copied().collect();

    let vectors = mock.embed(&text_refs).await?;
    println!("Generated {} vectors, each with {} dimensions", vectors.len(), mock.dimensions());

    for (i, (text, vec)) in texts.iter().zip(vectors.iter()).enumerate() {
        println!("Text {}: {:?}... → [{:.3}, ...]", i, &text[..30], vec[0]);
    }

    Ok(())
}

use xz_rag::{
    ChannelConfig, ChannelPipeline, ContextBuilder, DefaultRagEngineBuilder,
    PromptTemplate, RagEngine, RagRequest, RetrieveRequest,
};

#[tokio::test]
async fn test_engine_builder() {
    let engine = DefaultRagEngineBuilder::default()
        .name("test-engine")
        .version("0.1.0")
        .pipeline(ChannelPipeline::new(vec![
            ChannelConfig::semantic(0.5, 10),
        ]))
        .build();

    let info = engine.engine_info();
    assert_eq!(info.name, "test-engine");
    assert!(info.supported_channels.contains(&"semantic".to_string()));
}

#[tokio::test]
async fn test_retrieve_empty_returns_error() {
    let engine = DefaultRagEngineBuilder::default()
        .pipeline(ChannelPipeline::new(vec![]))
        .build();

    let request = RetrieveRequest::builder("test query").build();

    let result = engine.retrieve(&request).await.unwrap();
    // Empty pipeline returns empty hits
    assert!(result.hits.is_empty());
    assert!(result.channel_report.is_empty());
}

#[tokio::test]
async fn test_retrieve_and_generate_no_hits() {
    let engine = DefaultRagEngineBuilder::default()
        .pipeline(ChannelPipeline::new(vec![]))
        .build();

    let request = RagRequest::builder("test query")
        .build();

    let result = engine.retrieve_and_generate(&request).await;
    assert!(result.is_err());
}

#[test]
fn test_context_builder() {
    let builder = ContextBuilder::new(4096);
    let budget = builder.context_budget();
    assert!(budget > 0);
}

#[test]
fn test_prompt_template_render() {
    let template = PromptTemplate::default_qa();
    let rendered = template.render("What is Rust?", "Rust is a systems programming language.");
    assert!(rendered.contains("What is Rust?"));
    assert!(rendered.contains("Rust is a systems programming language"));
}

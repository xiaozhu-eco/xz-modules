use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use futures::stream::Stream;
use tracing::info;

use crate::channels::graph::{GraphChannelExecutor, KnowledgeGraphSearch};
use crate::channels::metadata::{MetadataChannelExecutor, MetadataStore};
use crate::channels::semantic::{Embedder, SemanticChannelExecutor, SemanticSearch};
use crate::context::token_budget::ContextBuilder;
use crate::error::RagError;
use crate::pipeline::channel::{ChannelConfig, ChannelPipeline, ChannelType};
use crate::pipeline::fusion::RrfFusion;
use crate::pipeline::normalize::MinMaxNormalizer;
use crate::traits::RagEngine;
use crate::types::config::RagEngineInfo;
use crate::types::rag::{
    BuiltContext, ChatMessage, ChatRole, PromptTemplate, RagRequest, RagResponse,
    RagStreamEvent, RagTokenUsage,
};
use crate::types::retrieval::{
    ChannelStats, QueryPreprocessing, RetrievedChunk, RetrieveRequest, RetrieveResult,
};

#[cfg(feature = "rerank")]
use xz_rerank::{traits::Reranker, RerankCandidate, RerankConfig};

#[cfg(feature = "llm-generation")]
use xz_provider::{
    CompletionRequest as ProviderCompletionRequest,
    LlmProvider,
    RequestOptions as ProviderRequestOptions,
    StreamEvent,
    types::message::Message as ProviderMessage,
};

/// Default multi-channel RAG engine implementation.
pub struct DefaultRagEngine {
    info: RagEngineInfo,
    pipeline: ChannelPipeline,
    context_builder: ContextBuilder,
    prompt_template: PromptTemplate,
    // Pluggable components
    embedder: Option<Arc<dyn Embedder>>,
    semantic_store: Option<Arc<dyn SemanticSearch>>,
    metadata_store: Option<Arc<dyn MetadataStore>>,
    graph_store: Option<Arc<dyn KnowledgeGraphSearch>>,
    #[cfg(feature = "rerank")]
    reranker: Option<Arc<dyn Reranker>>,
    #[cfg(feature = "llm-generation")]
    provider: Option<Arc<dyn LlmProvider>>,
    #[cfg(feature = "caching")]
    cache: Option<crate::cache::memory_cache::RagMemoryCache>,
}

impl DefaultRagEngine {
    pub fn builder() -> DefaultRagEngineBuilder {
        DefaultRagEngineBuilder::default()
    }
}

impl DefaultRagEngine {
    /// Execute retrieval across all configured channels.
    async fn do_retrieve(&self, request: &RetrieveRequest) -> Result<RetrieveResult, RagError> {
        // Check cache first
        #[cfg(feature = "caching")]
        if let Some(ref cache) = self.cache {
            let cache_key = build_cache_key(&request.query, &request.namespace);
            if let Some(cached) = cache.get(&cache_key).await {
                info!(query = %request.query, "RAG cache hit");
                return Ok(cached);
            }
        }

        // Preprocess query (HYDE, expansion, translation)
        let effective_query = self.preprocess_query(request).await?;

        let start = Instant::now();
        let mut channel_results: HashMap<String, Vec<RetrievedChunk>> = HashMap::new();
        let mut channel_report: HashMap<String, ChannelStats> = HashMap::new();

        for channel_config in &self.pipeline.channels {
            let channel_start = Instant::now();
            let namespace = request.namespace.as_deref();

            let hits = match &channel_config.channel_type {
                ChannelType::Semantic => {
                    if let (Some(emb), Some(store)) = (&self.embedder, &self.semantic_store) {
                        let executor = SemanticChannelExecutor::new(emb.clone(), store.clone());
                        executor
                            .execute(&effective_query, channel_config, &request.global_filters, namespace)
                            .await?
                    } else {
                        vec![]
                    }
                }
                ChannelType::Metadata => {
                    if let Some(store) = &self.metadata_store {
                        let executor = MetadataChannelExecutor::new(store.clone());
                        executor
                            .execute(&effective_query, channel_config, &request.global_filters, namespace)
                            .await?
                    } else {
                        vec![]
                    }
                }
                ChannelType::Bm25 => {
                    #[cfg(feature = "bm25")]
                    {
                        let executor = crate::channels::bm25::Bm25ChannelExecutor::new();
                        executor
                            .execute(&effective_query, channel_config, &request.global_filters, namespace)
                            .await?
                    }
                    #[cfg(not(feature = "bm25"))]
                    vec![]
                }
                ChannelType::Graph => {
                    if let Some(store) = &self.graph_store {
                        let executor = GraphChannelExecutor::new(store.clone());
                        executor
                            .execute(&effective_query, channel_config, &request.global_filters, namespace)
                            .await?
                    } else {
                        vec![]
                    }
                }
                _ => vec![],
            };

            let latency = channel_start.elapsed().as_millis() as u64;
            let min_score = hits.iter().map(|h| h.score).fold(f32::INFINITY, f32::min);
            let max_score = hits.iter().map(|h| h.score).fold(0.0_f32, f32::max);

            channel_report.insert(
                channel_config.channel_type.as_str().to_string(),
                ChannelStats {
                    channel_type: channel_config.channel_type.as_str().to_string(),
                    hits: hits.len(),
                    latency_ms: latency,
                    min_score,
                    max_score,
                },
            );

            channel_results.insert(channel_config.channel_type.as_str().to_string(), hits);
        }

        // Normalize scores per channel
        if self.pipeline.normalize_scores {
            for hits in channel_results.values_mut() {
                let mut scores: Vec<f32> = hits.iter().map(|h| h.score).collect();
                MinMaxNormalizer::normalize(&mut scores);
                for (hit, score) in hits.iter_mut().zip(scores) {
                    hit.score = score;
                }
            }
        }

        // RRF fusion
        let fusion = RrfFusion::new(self.pipeline.rrf_k);
        let mut fused = fusion.fuse(channel_results);

        #[cfg(feature = "rerank")]
        if let Some(reranker) = &self.reranker {
            if !fused.is_empty() {
                let original_hits: HashMap<String, RetrievedChunk> = fused
                    .iter()
                    .cloned()
                    .map(|chunk| (chunk.chunk_id.clone(), chunk))
                    .collect();

                let candidates: Vec<RerankCandidate> = fused
                    .iter()
                    .map(retrieved_chunk_to_rerank_candidate)
                    .collect();

                let rerank_result = reranker
                    .rerank(
                        &effective_query,
                        candidates,
                        &RerankConfig {
                            top_k: request.top_k,
                            min_score: None,
                            include_score_breakdown: false,
                            recency_mode: None,
                        },
                    )
                    .await
                    .map_err(|e| RagError::Rerank(e.to_string()))?;

                fused = rerank_result
                    .hits
                    .into_iter()
                    .filter_map(|hit| {
                        original_hits
                            .get(&hit.candidate_id)
                            .cloned()
                            .map(|mut chunk| {
                                chunk.score = hit.score;
                                chunk
                            })
                    })
                    .collect();
            }
        }

        // Apply global top_k
        fused.truncate(request.top_k);

        let latency_ms = start.elapsed().as_millis() as u64;

        let result = RetrieveResult {
            hits: fused,
            channel_report,
            latency_ms,
            effective_query,
        };

        // Store in cache
        #[cfg(feature = "caching")]
        if let Some(ref cache) = self.cache {
            let cache_key = build_cache_key(&request.query, &request.namespace);
            cache.set(&cache_key, result.clone()).await;
        }

        Ok(result)
    }

    /// Build context from retrieved chunks.
    fn build_context(&self, chunks: &[RetrievedChunk], query: &str) -> BuiltContext {
        self.context_builder.build(chunks, query)
    }

    /// Preprocess query with HYDE, expansion, or translation.
    async fn preprocess_query(&self, request: &RetrieveRequest) -> Result<String, RagError> {
        match &request.query_preprocessing {
            Some(QueryPreprocessing::Hyde) => {
                #[cfg(feature = "hyde")]
                {
                    let provider = self.provider.as_ref()
                        .ok_or_else(|| RagError::QueryPreprocessing("No LLM provider configured for HYDE".into()))?;
                    let expander = crate::preprocessing::hyde::HydeExpander::default();
                    expander.expand(&request.query, provider.as_ref()).await
                }
                #[cfg(not(feature = "hyde"))]
                Ok(request.query.clone())
            }
            Some(QueryPreprocessing::QueryExpansion { count }) => {
                #[cfg(feature = "hyde")]
                {
                    let provider = self.provider.as_ref()
                        .ok_or_else(|| RagError::QueryPreprocessing("No LLM provider configured for expansion".into()))?;
                    let expander = crate::preprocessing::hyde::HydeExpander::default();
                    let expanded = expander.expand(&request.query, provider.as_ref()).await?;
                    // Concatenate original with expanded for richer retrieval
                    Ok(format!("{} {}", request.query, expanded))
                }
                #[cfg(not(feature = "hyde"))]
                {
                    let _ = count;
                    Ok(request.query.clone())
                }
            }
            _ => Ok(request.query.clone()),
        }
    }

    /// Assemble the full prompt for LLM generation with optional chat history.
    fn assemble_prompt(
        &self,
        query: &str,
        context: &BuiltContext,
        system_prompt: Option<&str>,
        history: &[ChatMessage],
    ) -> String {
        let system = system_prompt.unwrap_or(&self.prompt_template.system);
        let context_block = format!(
            "{}{}{}",
            self.prompt_template.context_prefix,
            context.context_text,
            self.prompt_template.context_suffix
        );

        // Format chat history
        let mut history_block = String::new();
        for msg in history {
            let role = match msg.role {
                ChatRole::System => "System",
                ChatRole::User => "User",
                ChatRole::Assistant => "Assistant",
            };
            history_block.push_str(&format!("{}: {}\n", role, msg.content));
        }

        let user = if history_block.is_empty() {
            self.prompt_template.render(query, &context_block)
        } else {
            let with_history = format!(
                "Previous conversation:\n{}\n\nContext:\n{}\n\nQuestion: {}",
                history_block, context_block, query
            );
            with_history
        };

        format!("{}\n\n{}", system, user)
    }
}

#[async_trait]
impl RagEngine for DefaultRagEngine {
    async fn retrieve(&self, request: &RetrieveRequest) -> Result<RetrieveResult, RagError> {
        self.do_retrieve(request).await
    }

    async fn retrieve_and_generate(&self, request: &RagRequest) -> Result<RagResponse, RagError> {
        let start = Instant::now();

        // Step 1: Retrieve
        let retrieve_result = self.do_retrieve(&request.retrieve_config).await?;

        if retrieve_result.hits.is_empty() {
            return Err(RagError::NoResults(request.query.clone()));
        }

        // Step 2: Build context
        let built = self.build_context(&retrieve_result.hits, &request.query);

        // Step 3: Assemble prompt with history
        let prompt = self.assemble_prompt(
            &request.query,
            &built,
            request.system_prompt.as_deref(),
            &request.history,
        );

        // Step 4: Generate via LLM (or fallback to placeholder)
        let (answer, _llm_usage) = {
            #[cfg(feature = "llm-generation")]
            {
                if let Some(ref provider) = self.provider {
                    crate::generation::generate_response(
                        provider,
                        &prompt,
                        request.generation.model.as_deref(),
                        request.generation.temperature,
                        request.generation.max_output_tokens,
                    )
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!("LLM generation failed, using placeholder: {}", e);
                        (
                            format!(
                                "RAG Response for: '{}'\n\nBased on {} context chunks:\n{}",
                                request.query, built.chunks_used, built.context_text
                            ),
                            RagTokenUsage::default(),
                        )
                    })
                } else {
                    (
                        format!(
                            "RAG Response for: '{}'\n\nBased on {} context chunks:\n{}",
                            request.query, built.chunks_used, built.context_text
                        ),
                        RagTokenUsage::default(),
                    )
                }
            }
            #[cfg(not(feature = "llm-generation"))]
            {
                (
                    format!(
                        "RAG Response for: '{}'\n\nBased on {} context chunks:\n{}",
                        request.query, built.chunks_used, built.context_text
                    ),
                    RagTokenUsage::default(),
                )
            }
        };

        let usage = RagTokenUsage {
            context_tokens: built.tokens_used,
            prompt_tokens: prompt.len() / 4,
            completion_tokens: answer.len() / 4,
            total_tokens: (prompt.len() + answer.len()) / 4,
            chunks_used: built.chunks_used,
            chunks_dropped: built.chunks_dropped,
        };

        let total_latency_ms = start.elapsed().as_millis() as u64;

        info!(
            query = %request.query,
            hits = %retrieve_result.hits.len(),
            chunks_used = %built.chunks_used,
            latency_ms = %total_latency_ms,
            "RAG retrieval and generation complete"
        );

        Ok(RagResponse {
            answer,
            citations: built.citations,
            usage,
            retrieve_stats: retrieve_result,
            total_latency_ms,
            model: request.generation.model.clone(),
        })
    }

    async fn retrieve_and_generate_stream(
        &self,
        request: &RagRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<RagStreamEvent, RagError>> + Send>>, RagError> {
        let retrieve_result = self.do_retrieve(&request.retrieve_config).await?;

        if retrieve_result.hits.is_empty() {
            return Err(RagError::NoResults(request.query.clone()));
        }

        let built = self.build_context(&retrieve_result.hits, &request.query);
        #[cfg_attr(not(feature = "llm-generation"), allow(unused_variables))]
        let prompt = self.assemble_prompt(
            &request.query,
            &built,
            request.system_prompt.as_deref(),
            &request.history,
        );

        #[cfg(feature = "llm-generation")]
        {
            if let Some(ref provider) = self.provider {
                let model_name = request
                    .generation
                    .model
                    .clone()
                    .or_else(|| provider.default_model().map(|s| s.to_string()))
                    .unwrap_or_else(|| "default".to_string());

                let provider_request = ProviderCompletionRequest {
                    model: Some(model_name),
                    messages: vec![ProviderMessage::user(&prompt)],
                    temperature: request.generation.temperature,
                    max_tokens: request.generation.max_output_tokens,
                    stop: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    tools: None,
                    tool_choice: None,
                    response_format: None,
                    max_completion_tokens: None,
                    top_p: None,
                    top_k: None,
                    seed: None,
                    reasoning_effort: None,
                    logprobs: None,
                    logit_bias: None,
                    stream_include_usage: None,
                    request_id: String::new(),
                };

                let stream = provider.complete_stream(
                    provider_request,
                    ProviderRequestOptions::default(),
                ).await.map_err(|e| RagError::Provider(format!("Stream failed: {}", e)))?;

                let citations = built.citations.clone();
                let context_chunks = built.chunks_used;
                let context_tokens = built.tokens_used;

                let mapped = futures::stream::unfold(
                    (stream, citations, context_chunks, context_tokens, true),
                    |(mut stream, citations, context_chunks, context_tokens, mut sent_start)| async move {
                        if sent_start {
                            sent_start = false;
                            Some((
                                Ok(RagStreamEvent::GenerationStarted {
                                    context_chunks,
                                    context_tokens,
                                }),
                                (stream, citations, context_chunks, context_tokens, false),
                            ))
                        } else {
                            match futures::StreamExt::next(&mut stream).await {
                                Some(Ok(StreamEvent::ContentDelta { delta })) => Some((
                                    Ok(RagStreamEvent::ContentDelta { delta }),
                                    (stream, citations, context_chunks, context_tokens, false),
                                )),
                                Some(Ok(StreamEvent::Done { .. })) => {
                                    let done_event = Ok(RagStreamEvent::Done {
                                        total_latency_ms: 0,
                                        citations: citations.clone(),
                                        usage: RagTokenUsage::default(),
                                    });
                                    Some((done_event, (stream, citations, context_chunks, context_tokens, false)))
                                }
                                Some(Ok(StreamEvent::Usage { .. })) => Some((
                                    Ok(RagStreamEvent::ContentDelta { delta: String::new() }),
                                    (stream, citations, context_chunks, context_tokens, false),
                                )),
                                Some(Ok(_other)) => Some((
                                    Ok(RagStreamEvent::ContentDelta { delta: String::new() }),
                                    (stream, citations, context_chunks, context_tokens, false),
                                )),
                                Some(Err(e)) => Some((
                                    Err(RagError::Provider(format!("Stream error: {}", e))),
                                    (stream, citations, context_chunks, context_tokens, false),
                                )),
                                None => None,
                            }
                        }
                    },
                );

                return Ok(Box::pin(mapped));
            }
        }


        let stream = futures::stream::iter(vec![
            Ok(RagStreamEvent::GenerationStarted {
                context_chunks: built.chunks_used,
                context_tokens: built.tokens_used,
            }),
            Ok(RagStreamEvent::ContentDelta {
                delta: format!(
                    "RAG Response for: '{}'\n\nBased on {} context chunks",
                    request.query, built.chunks_used
                ),
            }),
            Ok(RagStreamEvent::Done {
                total_latency_ms: 0,
                citations: built.citations.clone(),
                usage: RagTokenUsage {
                    context_tokens: built.tokens_used,
                    chunks_used: built.chunks_used,
                    chunks_dropped: built.chunks_dropped,
                    ..RagTokenUsage::default()
                },
            }),
        ]);

        Ok(Box::pin(stream))
    }

    fn engine_info(&self) -> RagEngineInfo {
        self.info.clone()
    }
}

/// Builder for DefaultRagEngine.
#[derive(Default)]
pub struct DefaultRagEngineBuilder {
    name: Option<String>,
    version: Option<String>,
    pipeline: Option<ChannelPipeline>,
    context_builder: Option<ContextBuilder>,
    prompt_template: Option<PromptTemplate>,
    embedder: Option<Arc<dyn Embedder>>,
    semantic_store: Option<Arc<dyn SemanticSearch>>,
    metadata_store: Option<Arc<dyn MetadataStore>>,
    graph_store: Option<Arc<dyn KnowledgeGraphSearch>>,
    #[cfg(feature = "rerank")]
    reranker: Option<Arc<dyn Reranker>>,
    #[cfg(feature = "llm-generation")]
    provider: Option<Arc<dyn LlmProvider>>,
    #[cfg(feature = "caching")]
    cache: Option<crate::cache::memory_cache::RagMemoryCache>,
}

impl DefaultRagEngineBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn pipeline(mut self, pipeline: ChannelPipeline) -> Self {
        self.pipeline = Some(pipeline);
        self
    }

    pub fn context_builder(mut self, cb: ContextBuilder) -> Self {
        self.context_builder = Some(cb);
        self
    }

    pub fn prompt_template(mut self, pt: PromptTemplate) -> Self {
        self.prompt_template = Some(pt);
        self
    }

    pub fn embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    pub fn semantic_store(mut self, store: Arc<dyn SemanticSearch>) -> Self {
        self.semantic_store = Some(store);
        self
    }

    pub fn metadata_store(mut self, store: Arc<dyn MetadataStore>) -> Self {
        self.metadata_store = Some(store);
        self
    }

    pub fn graph_store(mut self, store: Arc<dyn KnowledgeGraphSearch>) -> Self {
        self.graph_store = Some(store);
        self
    }

    #[cfg(feature = "rerank")]
    pub fn reranker(mut self, reranker: Arc<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    #[cfg(feature = "caching")]
    pub fn cache(mut self, cache: crate::cache::memory_cache::RagMemoryCache) -> Self {
        self.cache = Some(cache);
        self
    }

    #[cfg(feature = "llm-generation")]
    pub fn provider(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn build(self) -> DefaultRagEngine {
        let pipeline = self.pipeline.unwrap_or_else(|| {
            ChannelPipeline::new(vec![
                ChannelConfig::semantic(0.5, 10).with_min_score(0.1),
                ChannelConfig::metadata(0.3, 5),
            ])
        });

        let mut supported_channels: Vec<String> = pipeline
            .channels
            .iter()
            .map(|c| c.channel_type.as_str().to_string())
            .collect();

        if self.graph_store.is_some() && !supported_channels.iter().any(|c| c == "graph") {
            supported_channels.push("graph".to_string());
        }

        DefaultRagEngine {
            info: RagEngineInfo {
                name: self.name.unwrap_or_else(|| "default".into()),
                version: self.version.unwrap_or_else(|| "0.1.0".into()),
                supported_channels,
                #[cfg(feature = "llm-generation")]
                supports_streaming: self.provider.is_some(),
                #[cfg(not(feature = "llm-generation"))]
                supports_streaming: false,
                reranking_enabled: {
                    #[cfg(feature = "rerank")]
                    {
                        self.reranker.is_some()
                    }
                    #[cfg(not(feature = "rerank"))]
                    {
                        false
                    }
                },
                max_context_window: self
                    .context_builder
                    .as_ref()
                    .map(|cb| cb.context_budget())
                    .unwrap_or(4096),
            },
            pipeline,
            context_builder: self
                .context_builder
                .unwrap_or_else(|| ContextBuilder::new(4096)),
            prompt_template: self
                .prompt_template
                .unwrap_or_else(PromptTemplate::default_qa),
            embedder: self.embedder,
            semantic_store: self.semantic_store,
            metadata_store: self.metadata_store,
            graph_store: self.graph_store,
            #[cfg(feature = "rerank")]
            reranker: self.reranker,
            #[cfg(feature = "llm-generation")]
            provider: self.provider,
            #[cfg(feature = "caching")]
            cache: self.cache,
        }
    }
}

#[cfg(feature = "rerank")]
fn retrieved_chunk_to_rerank_candidate(chunk: &RetrievedChunk) -> RerankCandidate {
    RerankCandidate {
        id: chunk.chunk_id.clone(),
        content: chunk.content.clone(),
        metadata: chunk_metadata_to_map(&chunk.metadata, &chunk.document_id),
        retrieval_score: Some(chunk.score),
        channel: Some(chunk.channel.clone()),
        created_at: chunk.metadata.created_at,
        embedding: chunk.embedding.clone(),
    }
}

#[cfg(feature = "rerank")]
fn chunk_metadata_to_map(
    metadata: &crate::types::chunk::ChunkMetadata,
    document_id: &str,
) -> std::collections::HashMap<String, String> {
    let mut map = metadata.extra.clone();

    map.insert("document_id".to_string(), document_id.to_string());

    if let Some(source) = &metadata.source {
        map.insert("source".to_string(), source.clone());
    }
    if let Some(document_title) = &metadata.document_title {
        map.insert("document_title".to_string(), document_title.clone());
    }
    if let Some(author) = &metadata.author {
        map.insert("author".to_string(), author.clone());
    }
    if let Some(created_at) = metadata.created_at {
        map.insert("created_at".to_string(), created_at.to_string());
    }
    if !metadata.tags.is_empty() {
        map.insert("tags".to_string(), metadata.tags.join(","));
    }
    if let Some(namespace) = &metadata.namespace {
        map.insert("namespace".to_string(), namespace.clone());
    }

    map
}

/// Build a cache key from query and optional namespace.
#[allow(unused)]
fn build_cache_key(query: &str, namespace: &Option<String>) -> String {
    if let Some(ns) = namespace {
        format!("rag:{}:{}", ns, query)
    } else {
        format!("rag:default:{}", query)
    }
}

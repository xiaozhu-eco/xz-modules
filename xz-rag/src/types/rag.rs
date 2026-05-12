use serde::{Deserialize, Serialize};

use super::retrieval::{RetrieveRequest, RetrieveResult};

// === RAG Request ===

/// End-to-end RAG request including retrieval, context, and generation config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagRequest {
    pub query: String,
    pub system_prompt: Option<String>,
    pub history: Vec<ChatMessage>,
    pub retrieve_config: RetrieveRequest,
    pub generation: RagGenerationConfig,
    pub context_config: Option<ContextConfig>,
    pub prompt_template: Option<String>,
    pub options: RequestOptions,
}

impl RagRequest {
    pub fn builder(query: impl Into<String>) -> RagRequestBuilder {
        RagRequestBuilder {
            query: query.into(),
            system_prompt: None,
            history: vec![],
            retrieve_config: RetrieveRequest::builder("").build(),
            generation: RagGenerationConfig::default(),
            context_config: None,
            prompt_template: None,
            options: RequestOptions::default(),
        }
    }
}

pub struct RagRequestBuilder {
    query: String,
    system_prompt: Option<String>,
    history: Vec<ChatMessage>,
    retrieve_config: RetrieveRequest,
    generation: RagGenerationConfig,
    context_config: Option<ContextConfig>,
    prompt_template: Option<String>,
    options: RequestOptions,
}

impl RagRequestBuilder {
    pub fn system_prompt(mut self, sp: impl Into<String>) -> Self {
        self.system_prompt = Some(sp.into());
        self
    }

    pub fn history(mut self, h: Vec<ChatMessage>) -> Self {
        self.history = h;
        self
    }

    pub fn retrieve_config(mut self, rc: RetrieveRequest) -> Self {
        self.retrieve_config = rc;
        self
    }

    pub fn generation(mut self, g: RagGenerationConfig) -> Self {
        self.generation = g;
        self
    }

    pub fn context_config(mut self, cc: ContextConfig) -> Self {
        self.context_config = Some(cc);
        self
    }

    pub fn prompt_template(mut self, pt: impl Into<String>) -> Self {
        self.prompt_template = Some(pt.into());
        self
    }

    pub fn options(mut self, opts: RequestOptions) -> Self {
        self.options = opts;
        self
    }

    pub fn build(self) -> RagRequest {
        RagRequest {
            query: self.query,
            system_prompt: self.system_prompt,
            history: self.history,
            retrieve_config: self.retrieve_config,
            generation: self.generation,
            context_config: self.context_config,
            prompt_template: self.prompt_template,
            options: self.options,
        }
    }
}

// === Chat Message ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

// === Request Options ===

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestOptions {
    pub namespace: Option<String>,
    pub timeout_ms: Option<u64>,
    pub retry_count: Option<u32>,
    pub stream: bool,
}

// === RAG Generation Config ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagGenerationConfig {
    pub max_context_tokens: usize,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<usize>,
    pub stream: bool,
}

impl Default for RagGenerationConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 4096,
            model: None,
            temperature: Some(0.7),
            max_output_tokens: Some(1024),
            stream: false,
        }
    }
}

// === Context Config ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_context_tokens: usize,
    pub system_prompt_reserve: usize,
    pub query_reserve: usize,
    pub output_reserve: usize,
    pub min_chunk_overlap: usize,
    pub citation_format: CitationFormat,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 4096,
            system_prompt_reserve: 256,
            query_reserve: 128,
            output_reserve: 512,
            min_chunk_overlap: 50,
            citation_format: CitationFormat::Numeric,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CitationFormat {
    Numeric,
    ChunkId,
    SourceName,
}

// === RAG Response ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub usage: RagTokenUsage,
    pub retrieve_stats: RetrieveResult,
    pub total_latency_ms: u64,
    pub model: Option<String>,
}

// === Citation ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub index: usize,
    pub chunk_id: String,
    pub content: String,
    pub document_title: Option<String>,
    pub score: f32,
    pub channel: String,
}

// === Token Usage ===

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RagTokenUsage {
    pub context_tokens: usize,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub chunks_used: usize,
    pub chunks_dropped: usize,
}

// === Prompt Template ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    pub system: String,
    pub user_template: String,
    pub context_prefix: String,
    pub chunk_format: String,
    pub context_suffix: String,
    pub citation_instruction: String,
}

impl PromptTemplate {
    pub fn default_qa() -> Self {
        Self {
            name: "default_qa".into(),
            system: "You are a helpful assistant. Answer the user's question based on the provided context.".into(),
            user_template: "Question: {query}\n\nContext:\n{context}\n\nAnswer:".into(),
            context_prefix: "Relevant information:\n".into(),
            chunk_format: "[{index}] {content}\n".into(),
            context_suffix: "\n".into(),
            citation_instruction: "Please cite sources using [N] notation when referencing the context.".into(),
        }
    }

    pub fn render(&self, query: &str, context: &str) -> String {
        self.user_template
            .replace("{query}", query)
            .replace("{context}", context)
    }
}

// === Streaming Event ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RagStreamEvent {
    RetrievalStarted { channel_count: usize },
    ChannelDone { channel: String, hits: usize, latency_ms: u64 },
    GenerationStarted { context_chunks: usize, context_tokens: usize },
    ContentDelta { delta: String },
    Citation { chunk_id: String, index: usize },
    Done { total_latency_ms: u64, citations: Vec<Citation>, usage: RagTokenUsage },
}

// === Built Context ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltContext {
    pub context_text: String,
    pub citations: Vec<Citation>,
    pub chunks_used: usize,
    pub chunks_dropped: usize,
    pub tokens_used: usize,
}

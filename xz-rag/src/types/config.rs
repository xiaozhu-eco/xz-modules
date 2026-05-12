use serde::{Deserialize, Serialize};

/// RAG engine information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagEngineInfo {
    pub name: String,
    pub version: String,
    pub supported_channels: Vec<String>,
    pub supports_streaming: bool,
    pub reranking_enabled: bool,
    pub max_context_window: usize,
}

/// RAG engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub engine: EngineSection,
    pub channels: ChannelSection,
    pub fusion: FusionSection,
    pub reranking: RerankingSection,
    pub context: ContextSection,
    pub chunking: ChunkingSection,
    pub query_preprocessing: QueryPreprocessingSection,
    pub prompt_templates: Vec<TemplateSection>,
    pub cache: CacheSection,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            engine: EngineSection::default(),
            channels: ChannelSection::default(),
            fusion: FusionSection::default(),
            reranking: RerankingSection::default(),
            context: ContextSection::default(),
            chunking: ChunkingSection::default(),
            query_preprocessing: QueryPreprocessingSection::default(),
            prompt_templates: vec![],
            cache: CacheSection::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSection {
    pub name: String,
    pub namespace: Option<String>,
}

impl Default for EngineSection {
    fn default() -> Self {
        Self {
            name: "default".into(),
            namespace: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSection {
    pub semantic: Option<ChannelDef>,
    pub bm25: Option<ChannelDef>,
    pub metadata: Option<ChannelDef>,
}

impl Default for ChannelSection {
    fn default() -> Self {
        Self {
            semantic: Some(ChannelDef { weight: 0.5, top_k: 10, min_score: Some(0.1) }),
            bm25: None,
            metadata: Some(ChannelDef { weight: 0.2, top_k: 5, min_score: None }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDef {
    pub weight: f32,
    pub top_k: usize,
    pub min_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionSection {
    pub algorithm: String,
    pub rrf_k: usize,
    pub normalize_scores: bool,
}

impl Default for FusionSection {
    fn default() -> Self {
        Self {
            algorithm: "rrf".into(),
            rrf_k: 60,
            normalize_scores: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingSection {
    pub enabled: bool,
    pub reranker: Option<String>,
}

impl Default for RerankingSection {
    fn default() -> Self {
        Self {
            enabled: false,
            reranker: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    pub max_context_tokens: usize,
    pub system_prompt_reserve: usize,
    pub query_reserve: usize,
    pub output_reserve: usize,
    pub citation_format: String,
    pub chunk_overlap: usize,
    pub separator: String,
}

impl Default for ContextSection {
    fn default() -> Self {
        Self {
            max_context_tokens: 4096,
            system_prompt_reserve: 256,
            query_reserve: 128,
            output_reserve: 512,
            citation_format: "numeric".into(),
            chunk_overlap: 50,
            separator: "\n---\n".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingSection {
    pub default_strategy: String,
    pub recursive: Option<ChunkerDef>,
    pub fixed: Option<FixedChunkerDef>,
}

impl Default for ChunkingSection {
    fn default() -> Self {
        Self {
            default_strategy: "recursive".into(),
            recursive: Some(ChunkerDef {
                chunk_size: 512,
                overlap: 50,
                separators: vec!["\n\n".into(), "\n".into(), ". ".into(), " ".into()],
            }),
            fixed: Some(FixedChunkerDef {
                chunk_size: 512,
                overlap: 50,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerDef {
    pub chunk_size: usize,
    pub overlap: usize,
    pub separators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedChunkerDef {
    pub chunk_size: usize,
    pub overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPreprocessingSection {
    pub hyde: Option<HydeDef>,
    pub query_expansion: Option<QueryExpansionDef>,
}

impl Default for QueryPreprocessingSection {
    fn default() -> Self {
        Self {
            hyde: None,
            query_expansion: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydeDef {
    pub prompt_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionDef {
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSection {
    pub name: String,
    pub system: String,
    pub user_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSection {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
}

impl Default for CacheSection {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_seconds: 3600,
            max_entries: 1000,
        }
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::chunk::ChunkMetadata;

use crate::pipeline::channel::ChannelConfig;

// === Retrieval Request ===

/// Multi-channel retrieval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveRequest {
    pub query: String,
    pub channels: Vec<ChannelConfig>,
    pub global_filters: Vec<StructuredFilter>,
    pub top_k: usize,
    pub namespace: Option<String>,
    pub include_embeddings: bool,
    pub query_preprocessing: Option<QueryPreprocessing>,
}

impl RetrieveRequest {
    pub fn builder(query: impl Into<String>) -> RetrieveRequestBuilder {
        RetrieveRequestBuilder {
            query: query.into(),
            channels: vec![ChannelConfig::semantic(0.5, 10)],
            global_filters: vec![],
            top_k: 10,
            namespace: None,
            include_embeddings: false,
            query_preprocessing: None,
        }
    }
}

pub struct RetrieveRequestBuilder {
    query: String,
    channels: Vec<ChannelConfig>,
    global_filters: Vec<StructuredFilter>,
    top_k: usize,
    namespace: Option<String>,
    include_embeddings: bool,
    query_preprocessing: Option<QueryPreprocessing>,
}

impl RetrieveRequestBuilder {
    pub fn channels(mut self, channels: Vec<ChannelConfig>) -> Self {
        self.channels = channels;
        self
    }

    pub fn top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn build(self) -> RetrieveRequest {
        RetrieveRequest {
            query: self.query,
            channels: self.channels,
            global_filters: self.global_filters,
            top_k: self.top_k,
            namespace: self.namespace,
            include_embeddings: self.include_embeddings,
            query_preprocessing: self.query_preprocessing,
        }
    }
}

// === Query Preprocessing ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryPreprocessing {
    None,
    Hyde,
    QueryExpansion { count: usize },
    TranslateToEnglish,
}

// === Structured Filter ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructuredFilter {
    MetadataEq { key: String, value: String },
    MetadataIn { key: String, values: Vec<String> },
    MetadataNe { key: String, value: String },
    MetadataExists { key: String },
    SqlFilter(String),
    And(Box<StructuredFilter>, Box<StructuredFilter>),
    Or(Box<StructuredFilter>, Box<StructuredFilter>),
}

// === Retrieve Result ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveResult {
    pub hits: Vec<RetrievedChunk>,
    pub channel_report: HashMap<String, ChannelStats>,
    pub latency_ms: u64,
    pub effective_query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub chunk_id: String,
    pub document_id: String,
    pub content: String,
    pub score: f32,
    pub channel: String,
    pub channel_score: f32,
    pub metadata: ChunkMetadata,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel_type: String,
    pub hits: usize,
    pub latency_ms: u64,
    pub min_score: f32,
    pub max_score: f32,
}

// === Search Result (from embed/types) ===

/// Re-export compatible search result for channel executors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

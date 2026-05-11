use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Channel type identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChannelType {
    Semantic,
    Metadata,
    Bm25,
    FullText,
    Graph,
    Custom(String),
}

impl ChannelType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Semantic => "semantic",
            Self::Metadata => "metadata",
            Self::Bm25 => "bm25",
            Self::FullText => "fulltext",
            Self::Graph => "graph",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Per-channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub channel_type: ChannelType,
    pub weight: f32,
    pub top_k: usize,
    pub min_score: Option<f32>,
    pub params: HashMap<String, serde_json::Value>,
}

impl ChannelConfig {
    pub fn new(channel_type: ChannelType, weight: f32, top_k: usize) -> Self {
        Self {
            channel_type,
            weight,
            top_k,
            min_score: None,
            params: HashMap::new(),
        }
    }

    pub fn semantic(weight: f32, top_k: usize) -> Self {
        Self::new(ChannelType::Semantic, weight, top_k)
    }

    pub fn metadata(weight: f32, top_k: usize) -> Self {
        Self::new(ChannelType::Metadata, weight, top_k)
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = Some(min_score);
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
}

/// Multi-channel pipeline orchestrating retrieval and fusion.
#[derive(Debug, Clone)]
pub struct ChannelPipeline {
    pub channels: Vec<ChannelConfig>,
    pub rrf_k: usize,
    pub normalize_scores: bool,
}

impl ChannelPipeline {
    pub fn new(channels: Vec<ChannelConfig>) -> Self {
        Self {
            channels,
            rrf_k: 60,
            normalize_scores: true,
        }
    }

    pub fn with_rrf_k(mut self, k: usize) -> Self {
        self.rrf_k = k;
        self
    }

    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize_scores = normalize;
        self
    }
}

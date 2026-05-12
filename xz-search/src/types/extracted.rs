use serde::{Deserialize, Serialize};

/// 提取的内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    /// URL
    pub url: String,
    /// 页面标题
    pub title: String,
    /// 正文内容（markdown）
    pub content: String,
    /// 内容长度（字符数）
    pub content_length: usize,
    /// 摘要
    pub excerpt: String,
    /// 作者
    pub author: Option<String>,
    /// 发布时间
    pub published_at: Option<u64>,
    /// 使用的提取器
    pub extractor: String,
    /// 提取延迟（毫秒）
    pub latency_ms: u64,
}

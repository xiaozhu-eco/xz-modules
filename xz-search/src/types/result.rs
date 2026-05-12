use serde::{Deserialize, Serialize};

use crate::types::ExtractedContent;

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 原始查询
    pub query: String,
    /// 搜索结果条目
    pub items: Vec<SearchItem>,
    /// 搜索结果总数
    pub total_results: u64,
    /// 总延迟（毫秒）
    pub latency_ms: u64,
    /// 是否来自缓存
    pub cached: bool,
    /// 使用的搜索引擎列表
    pub engines_used: Vec<String>,
    /// 重写后的查询
    pub rewritten_query: Option<String>,
}

/// 搜索结果条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    /// 标题
    pub title: String,
    /// URL
    pub url: String,
    /// 摘要/片段
    pub snippet: String,
    /// 来源引擎
    pub source: String,
    /// 发布时间（epoch seconds）
    pub published_at: Option<u64>,
    /// 搜索结果得分 [0, 1]
    pub score: f32,
    /// 来源域名
    pub domain: String,
    /// 语言检测
    pub detected_language: Option<String>,
    /// 提取的正文内容
    pub extracted_content: Option<ExtractedContent>,
}

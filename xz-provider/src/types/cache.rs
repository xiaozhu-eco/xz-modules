use serde::{Deserialize, Serialize};

/// Prompt Caching 控制标记
///
/// Anthropic 风格：显式标记缓存断点。
/// OpenAI 自动管理缓存，不需要客户端标记。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheControl {
    /// 标记此消息为缓存断点（Anthropic 风格）
    #[serde(rename = "ephemeral")]
    Ephemeral,
}

/// 缓存命中信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    /// 命中的缓存 token 数
    pub cached_tokens: u32,
    /// 因缓存而节省的金额（美元）
    pub cache_saved_cost: f64,
}

use serde::{Deserialize, Serialize};

/// 批量嵌入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbedRequest {
    /// 文本列表
    pub texts: Vec<String>,
    /// 请求来源标签（用于日志/指标分组）
    pub source: Option<String>,
    /// 优先级（0 = 最低，255 = 最高）
    pub priority: u8,
}

/// 批量嵌入响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbedResponse {
    /// 生成的向量（顺序与 texts 一致）
    pub vectors: Vec<Vec<f32>>,
    /// 使用的模型名称
    pub model: String,
    /// 总 token 用量
    pub total_tokens: u32,
    /// 总延迟（毫秒）
    pub latency_ms: u64,
    /// 是否有文本被截断
    pub truncated: Vec<bool>,
}

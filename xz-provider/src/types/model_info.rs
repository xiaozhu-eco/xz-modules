use serde::{Deserialize, Serialize};

/// 模型信息 —— 路由层据此做能力感知路由
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub provider: Option<String>,
    pub capabilities: ModelCapabilities,
    pub pricing: ModelPricing,
    pub limits: ModelLimits,
}

/// 模型能力声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub context_window: usize,
    pub max_output_tokens: usize,

    #[serde(default)]
    pub supports_tool_calling: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_streaming: bool,
    #[serde(default)]
    pub supports_structured_output: bool,
    #[serde(default)]
    pub supports_prompt_caching: bool,
    #[serde(default)]
    pub supports_thinking: bool,

    #[serde(default)]
    pub input_modalities: Vec<Modality>,
    #[serde(default)]
    pub output_modalities: Vec<Modality>,
}

/// 模态类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Modality {
    #[serde(rename = "Text")]
    Text,
    #[serde(rename = "Image")]
    Image,
    #[serde(rename = "Audio")]
    Audio,
    #[serde(rename = "Video")]
    Video,
}

/// 模型定价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// 每百万输入 token 价格（美元）
    pub input_per_million: f64,
    /// 每百万输出 token 价格（美元）
    pub output_per_million: f64,
    /// 缓存命中的每百万 token 价格
    #[serde(default)]
    pub cache_read_per_million: f64,
}

/// 模型限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimits {
    /// 最大批量请求数
    #[serde(default)]
    pub max_batch_size: usize,
    /// 每分钟请求数限制
    #[serde(default)]
    pub rpm: Option<u32>,
    /// 每分钟 Token 数限制
    #[serde(default)]
    pub tpm: Option<u32>,
}

/// 能力需求 —— 上层用此结构告诉路由层需要什么能力
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityRequest {
    pub tool_calling: bool,
    pub vision: bool,
    pub structured_output: bool,
    pub thinking: bool,
    pub min_context_window: Option<usize>,
    pub max_output_tokens: Option<usize>,
}

impl ModelInfo {
    /// 检查模型是否满足能力需求
    pub fn satisfies(&self, req: &CapabilityRequest) -> bool {
        if req.tool_calling && !self.capabilities.supports_tool_calling {
            return false;
        }
        if req.vision && !self.capabilities.supports_vision {
            return false;
        }
        if req.structured_output && !self.capabilities.supports_structured_output {
            return false;
        }
        if req.thinking && !self.capabilities.supports_thinking {
            return false;
        }
        if let Some(min_ctx) = req.min_context_window {
            if self.capabilities.context_window < min_ctx {
                return false;
            }
        }
        if let Some(max_out) = req.max_output_tokens {
            if self.capabilities.max_output_tokens < max_out {
                return false;
            }
        }
        true
    }

    /// 估算文本的 token 数（基于启发式近似）
    ///
    /// 用于上下文窗口预算管理——发送请求前判断 prompt 是否超限。
    /// 不需要精确到个位，±5% 即可满足预算控制需求。
    ///
    /// 实现策略：
    /// - 英文约 4 字符/token，中文约 1.5 字符/token
    /// - 混合文本取加权平均
    pub fn estimate_tokens(&self, text: &str) -> usize {
        let char_count = text.chars().count();
        let has_cjk = text
            .chars()
            .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c));
        let ratio = if has_cjk { 1.5 } else { 4.0 };
        (char_count as f64 / ratio).ceil() as usize
    }

    /// 计算剩余可用 token 空间
    pub fn remaining_capacity(&self, prompt_tokens: usize) -> usize {
        self.capabilities
            .context_window
            .saturating_sub(prompt_tokens)
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            context_window: 4096,
            max_output_tokens: 2048,
            supports_tool_calling: false,
            supports_vision: false,
            supports_streaming: false,
            supports_structured_output: false,
            supports_prompt_caching: false,
            supports_thinking: false,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
        }
    }
}

impl Default for ModelPricing {
    fn default() -> Self {
        Self {
            input_per_million: 0.0,
            output_per_million: 0.0,
            cache_read_per_million: 0.0,
        }
    }
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            max_batch_size: 1,
            rpm: None,
            tpm: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model(
        name: &str,
        tool_calling: bool,
        vision: bool,
        ctx: usize,
        max_out: usize,
    ) -> ModelInfo {
        ModelInfo {
            name: name.to_owned(),
            display_name: None,
            provider: None,
            capabilities: ModelCapabilities {
                context_window: ctx,
                max_output_tokens: max_out,
                supports_tool_calling: tool_calling,
                supports_vision: vision,
                ..Default::default()
            },
            pricing: ModelPricing::default(),
            limits: ModelLimits::default(),
        }
    }

    #[test]
    fn test_satisfies_empty_requirements() {
        let model = make_model("test", false, false, 4096, 2048);
        assert!(model.satisfies(&CapabilityRequest::default()));
    }

    #[test]
    fn test_satisfies_tool_calling() {
        let model = make_model("test", true, false, 4096, 2048);
        let req = CapabilityRequest { tool_calling: true, ..Default::default() };
        assert!(model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_missing_tool_calling() {
        let model = make_model("test", false, false, 4096, 2048);
        let req = CapabilityRequest { tool_calling: true, ..Default::default() };
        assert!(!model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_vision() {
        let model = make_model("test", false, true, 4096, 2048);
        let req = CapabilityRequest { vision: true, ..Default::default() };
        assert!(model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_missing_vision() {
        let model = make_model("test", false, false, 4096, 2048);
        let req = CapabilityRequest { vision: true, ..Default::default() };
        assert!(!model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_structured_output() {
        let model = make_model("test", false, false, 4096, 2048);
        let mut req = CapabilityRequest::default();
        req.structured_output = false;
        assert!(model.satisfies(&req));
        req.structured_output = true;
        assert!(!model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_thinking() {
        let model = make_model("test", false, false, 4096, 2048);
        let mut req = CapabilityRequest::default();
        req.thinking = true;
        assert!(!model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_min_context_window() {
        let model = make_model("test", false, false, 8000, 2048);
        let req = CapabilityRequest {
            min_context_window: Some(4000),
            ..Default::default()
        };
        assert!(model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_insufficient_context_window() {
        let model = make_model("test", false, false, 1000, 2048);
        let req = CapabilityRequest {
            min_context_window: Some(4000),
            ..Default::default()
        };
        assert!(!model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_max_output_tokens() {
        let model = make_model("test", false, false, 4096, 8000);
        let req = CapabilityRequest {
            max_output_tokens: Some(4000),
            ..Default::default()
        };
        assert!(model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_insufficient_max_output() {
        let model = make_model("test", false, false, 4096, 1000);
        let req = CapabilityRequest {
            max_output_tokens: Some(4000),
            ..Default::default()
        };
        assert!(!model.satisfies(&req));
    }

    #[test]
    fn test_satisfies_all_features() {
        let model = make_model("test", true, true, 128000, 4096);
        let req = CapabilityRequest {
            tool_calling: true,
            vision: true,
            structured_output: false,
            thinking: false,
            min_context_window: Some(32000),
            max_output_tokens: Some(2048),
        };
        assert!(model.satisfies(&req));
    }

    #[test]
    fn test_default_capabilities() {
        let caps = ModelCapabilities::default();
        assert_eq!(caps.context_window, 4096);
        assert_eq!(caps.max_output_tokens, 2048);
        assert!(!caps.supports_tool_calling);
        assert!(!caps.supports_vision);
    }

    #[test]
    fn test_default_pricing() {
        let pricing = ModelPricing::default();
        assert_eq!(pricing.input_per_million, 0.0);
        assert_eq!(pricing.output_per_million, 0.0);
    }

    #[test]
    fn test_default_limits() {
        let limits = ModelLimits::default();
        assert_eq!(limits.max_batch_size, 1);
        assert!(limits.rpm.is_none());
        assert!(limits.tpm.is_none());
    }

    #[test]
    fn test_model_info_serde() {
        let model = make_model("gpt-4", true, false, 8192, 4096);
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "gpt-4");
        assert!(deserialized.capabilities.supports_tool_calling);
    }

    #[test]
    fn test_capability_request_default() {
        let req = CapabilityRequest::default();
        assert!(!req.tool_calling);
        assert!(!req.vision);
        assert!(req.min_context_window.is_none());
    }

    #[test]
    fn test_estimate_tokens_english() {
        let model = make_model("test", false, false, 4096, 2048);
        let tokens = model.estimate_tokens("hello world");
        assert!(tokens > 0);
        assert!(tokens <= 4); // 11 chars / 4 ≈ 3
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let model = make_model("test", false, false, 4096, 2048);
        let tokens = model.estimate_tokens("你好世界");
        assert!(tokens > 0);
        assert!(tokens <= 4); // 4 chars / 1.5 ≈ 3
    }

    #[test]
    fn test_remaining_capacity() {
        let model = make_model("test", false, false, 4096, 2048);
        assert_eq!(model.remaining_capacity(1000), 3096);
        assert_eq!(model.remaining_capacity(5000), 0); // saturated
    }
}

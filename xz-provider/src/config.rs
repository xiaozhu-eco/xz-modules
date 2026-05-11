use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{
    ModelCapabilities, ModelInfo, ModelLimits, ModelPricing,
};

/// Provider 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    OpenAi,
    Claude,
    Local,
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型名称
    pub name: String,

    /// 可读名称（可选）
    #[serde(default)]
    pub display_name: Option<String>,

    /// 能力声明
    #[serde(default)]
    pub capabilities: ModelCapabilities,

    /// 价格信息
    #[serde(default)]
    pub pricing: ModelPricing,

    /// 速率限制
    #[serde(default)]
    pub limits: ModelLimits,
}

impl From<ModelConfig> for ModelInfo {
    fn from(cfg: ModelConfig) -> Self {
        ModelInfo {
            name: cfg.name,
            display_name: cfg.display_name,
            provider: None,
            capabilities: cfg.capabilities,
            pricing: cfg.pricing,
            limits: cfg.limits,
        }
    }
}

/// Provider 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDefinition {
    pub provider_type: ProviderType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    pub models: Vec<ModelConfig>,
}

/// 路由规则 — 按用途将请求映射到指定 Provider + 模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub model: String,
    pub provider: Option<String>,

    #[serde(default)]
    pub temperature: Option<f32>,

    #[serde(default)]
    pub max_tokens: Option<usize>,

    /// 回退链：主模型失败后按顺序尝试
    #[serde(default)]
    pub fallback: Vec<FallbackEntry>,
}

/// 回退条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEntry {
    pub model: String,
    pub provider: Option<String>,

    /// 回退触发条件
    #[serde(default)]
    pub condition: FallbackCondition,
}

/// 回退触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackCondition {
    /// 总是回退
    #[serde(rename = "always")]
    Always,
    /// 仅限限流时回退
    #[serde(rename = "rate_limit_only")]
    RateLimitOnly,
    /// 特定 HTTP 状态码时回退
    #[serde(rename = "error_status")]
    ErrorStatus(Vec<u16>),
}

impl Default for FallbackCondition {
    fn default() -> Self {
        Self::Always
    }
}

/// Provider 配置（v2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 默认模型名称
    pub default_model: Option<String>,

    /// 各 Provider 定义
    pub providers: HashMap<String, ProviderDefinition>,

    /// 按用途的命名路由
    #[serde(default)]
    pub routing: HashMap<String, RouteRule>,
}

impl ProviderConfig {
    /// 从 JSON 字符串加载
    pub fn from_json(json: &str) -> Result<Self, crate::error::ProviderError> {
        let config: Self =
            serde_json::from_str(json).map_err(|e| crate::error::ProviderError::Config(e.to_string()))?;
        config.validate()
    }

    /// 从 YAML 字符串加载
    pub fn from_yaml(yaml: &str) -> Result<Self, crate::error::ProviderError> {
        let yaml_interpolated = Self::interpolate_env(yaml);
        let config: Self = serde_yaml::from_str(&yaml_interpolated)
            .map_err(|e| crate::error::ProviderError::Config(e.to_string()))?;
        config.validate()
    }

    /// 从 JSON 文件加载
    pub async fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, crate::error::ProviderError> {
        let content = tokio::fs::read_to_string(path.as_ref())
            .await
            .map_err(|e| crate::error::ProviderError::Config(format!("读取配置文件失败: {e}")))?;
        Self::from_json(&content)
    }

    /// 从 YAML 文件加载
    pub async fn from_yaml_file(path: impl AsRef<std::path::Path>) -> Result<Self, crate::error::ProviderError> {
        let content = tokio::fs::read_to_string(path.as_ref())
            .await
            .map_err(|e| crate::error::ProviderError::Config(format!("读取配置文件失败: {e}")))?;
        Self::from_yaml(&content)
    }

    /// 展开环境变量引用 ${VAR_NAME}
    fn interpolate_env(input: &str) -> String {
        let mut result = input.to_string();
        // 匹配 ${VAR_NAME} 或 ${VAR_NAME:-default}
        let re = regex_lite::Regex::new(r"\$\{([^:}]+)(?::-(.*?))?\}").ok();
        if let Some(re) = re {
            for caps in re.captures_iter(input) {
                let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let default_val = caps.get(2).map(|m| m.as_str());
                let value = std::env::var(var_name)
                    .ok()
                    .or_else(|| default_val.map(|s| s.to_string()))
                    .unwrap_or_default();
                result = result.replace(caps.get(0).map(|m| m.as_str()).unwrap_or(""), &value);
            }
        }
        result
    }

    /// 验证配置合法性
    fn validate(self) -> Result<Self, crate::error::ProviderError> {
        for (name, def) in &self.providers {
            if def.models.is_empty() {
                return Err(crate::error::ProviderError::Config(format!(
                    "Provider '{name}' 没有定义任何模型"
                )));
            }
            if def.provider_type == ProviderType::OpenAi || def.provider_type == ProviderType::Claude {
                if def.api_key.as_ref().map_or(true, |k| k.is_empty()) {
                    return Err(crate::error::ProviderError::Config(format!(
                        "Provider '{name}' 缺少 api_key"
                    )));
                }
            }
        }
        Ok(self)
    }

    /// 收集所有模型信息（用于路由层注册）
    pub fn collect_models(&self) -> Vec<ModelInfo> {
        let mut models = Vec::new();
        for (provider_name, def) in &self.providers {
            for mc in &def.models {
                let mut info = ModelInfo::from(mc.clone());
                info.provider = Some(provider_name.clone());
                models.push(info);
            }
        }
        models
    }
}

/// 配置热更新监听器
pub trait ConfigWatcher: Send + Sync {
    /// 返回配置变更流
    fn watch(&self) -> futures::stream::BoxStream<'static, ProviderConfig>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_openai_json() -> &'static str {
        r#"{
            "default_model": "gpt-4",
            "providers": {
                "openai": {
                    "provider_type": "open_ai",
                    "api_key": "sk-test",
                    "models": [
                        {
                            "name": "gpt-4",
                            "capabilities": {
                                "context_window": 128000,
                                "max_output_tokens": 4096,
                                "supports_tool_calling": true
                            }
                        }
                    ]
                }
            }
        }"#
    }

    #[test]
    fn test_from_json_valid() {
        let config = ProviderConfig::from_json(valid_openai_json()).unwrap();
        assert_eq!(config.default_model.unwrap(), "gpt-4");
        assert!(config.providers.contains_key("openai"));
        assert_eq!(config.providers["openai"].provider_type, ProviderType::OpenAi);
        assert_eq!(config.providers["openai"].models.len(), 1);
        assert_eq!(config.providers["openai"].models[0].name, "gpt-4");
    }

    #[test]
    fn test_from_json_invalid_syntax() {
        let result = ProviderConfig::from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_json_empty_models() {
        let json = r#"{
            "providers": {
                "test": {
                    "provider_type": "open_ai",
                    "api_key": "sk-test",
                    "models": []
                }
            }
        }"#;
        let result = ProviderConfig::from_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{}", err).contains("没有定义任何模型"));
    }

    #[test]
    fn test_from_json_missing_api_key() {
        let json = r#"{
            "providers": {
                "test": {
                    "provider_type": "open_ai",
                    "models": [{"name": "gpt-4", "capabilities": {"context_window": 100, "max_output_tokens": 100}}]
                }
            }
        }"#;
        let result = ProviderConfig::from_json(json);
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("api_key"), "error: {}", err);
    }

    #[test]
    fn test_from_json_local_no_api_key_needed() {
        let json = r#"{
            "providers": {
                "local": {
                    "provider_type": "local",
                    "models": [{"name": "llama3", "capabilities": {"context_window": 4096, "max_output_tokens": 2048}}]
                }
            }
        }"#;
        let config = ProviderConfig::from_json(json).unwrap();
        assert!(config.providers.contains_key("local"));
    }

    #[test]
    fn test_from_yaml_valid() {
        let yaml = r#"
default_model: gpt-4
providers:
  openai:
    provider_type: open_ai
    api_key: sk-test
    models:
      - name: gpt-4
        capabilities:
          context_window: 128000
          max_output_tokens: 4096
"#;
        let config = ProviderConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.default_model.unwrap(), "gpt-4");
        assert!(config.providers.contains_key("openai"));
    }

    #[test]
    fn test_from_yaml_invalid() {
        let result = ProviderConfig::from_yaml(": bad yaml : :");
        assert!(result.is_err());
    }

    #[test]
    fn test_collect_models() {
        let config = ProviderConfig::from_json(valid_openai_json()).unwrap();
        let models = config.collect_models();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "gpt-4");
        assert_eq!(models[0].provider.as_deref(), Some("openai"));
        assert!(models[0].capabilities.supports_tool_calling);
    }

    #[test]
    fn test_collect_models_multiple_providers() {
        let json = r#"{
            "providers": {
                "p1": {
                    "provider_type": "open_ai",
                    "api_key": "k1",
                    "models": [{"name": "m1", "capabilities": {"context_window": 100, "max_output_tokens": 100}}]
                },
                "p2": {
                    "provider_type": "local",
                    "models": [{"name": "m2", "capabilities": {"context_window": 100, "max_output_tokens": 100}}, {"name": "m3", "capabilities": {"context_window": 100, "max_output_tokens": 100}}]
                }
            }
        }"#;
        let config = ProviderConfig::from_json(json).unwrap();
        let models = config.collect_models();
        assert_eq!(models.len(), 3);
        assert!(models.iter().any(|m| m.provider.as_deref() == Some("p1")));
        assert!(models.iter().any(|m| m.provider.as_deref() == Some("p2")));
    }

    #[test]
    fn test_interpolate_env_no_vars() {
        let input = "hello world";
        let result = ProviderConfig::interpolate_env(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_interpolate_env_with_default() {
        let input = r#"api_key: ${MY_KEY:-default_key}"#;
        let result = ProviderConfig::interpolate_env(input);
        assert_eq!(result, "api_key: default_key");
    }

    #[test]
    fn test_model_config_to_model_info() {
        let cfg = ModelConfig {
            name: "gpt-4".into(),
            display_name: Some("GPT-4".into()),
            capabilities: ModelCapabilities {
                context_window: 8192,
                max_output_tokens: 4096,
                ..Default::default()
            },
            pricing: ModelPricing {
                input_per_million: 30.0,
                output_per_million: 60.0,
                ..Default::default()
            },
            limits: ModelLimits::default(),
        };
        let info: ModelInfo = cfg.into();
        assert_eq!(info.name, "gpt-4");
        assert_eq!(info.display_name.unwrap(), "GPT-4");
        assert_eq!(info.capabilities.context_window, 8192);
        assert_eq!(info.pricing.input_per_million, 30.0);
        assert!(info.provider.is_none());
    }

    #[test]
    fn test_route_rule_default_fallback() {
        let rule = RouteRule {
            model: "gpt-4".into(),
            provider: Some("openai".into()),
            temperature: None,
            max_tokens: None,
            fallback: vec![],
        };
        assert_eq!(rule.model, "gpt-4");
    }

    #[test]
    fn test_fallback_condition_default() {
        let cond: FallbackCondition = Default::default();
        assert!(matches!(cond, FallbackCondition::Always));
    }

    #[test]
    fn test_config_routing() {
        let json = r#"{
            "default_model": "gpt-4",
            "providers": {
                "openai": {
                    "provider_type": "open_ai",
                    "api_key": "sk-test",
                    "models": [{"name": "gpt-4", "capabilities": {"context_window": 100, "max_output_tokens": 100}}]
                }
            },
            "routing": {
                "chat": {
                    "model": "gpt-4",
                    "provider": "openai"
                }
            }
        }"#;
        let config = ProviderConfig::from_json(json).unwrap();
        assert!(config.routing.contains_key("chat"));
        assert_eq!(config.routing["chat"].model, "gpt-4");
    }

    #[test]
    fn test_provider_type_serde() {
        assert_eq!(serde_json::to_string(&ProviderType::OpenAi).unwrap(), r#""open_ai""#);
        assert_eq!(serde_json::to_string(&ProviderType::Claude).unwrap(), r#""claude""#);
        assert_eq!(serde_json::to_string(&ProviderType::Local).unwrap(), r#""local""#);
    }
}

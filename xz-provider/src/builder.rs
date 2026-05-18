use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{ConfigWatcher, ProviderConfig, ProviderType};
use crate::error::{ProviderError, RetryStrategy};
use crate::providers::{LocalProvider, OpenAiProvider};
#[cfg(feature = "claude")]
use crate::providers::ClaudeProvider;
use crate::router::ProviderRouter;
use crate::traits::LlmProvider;

/// Provider 构建器
pub struct ProviderBuilder {
    config: Option<ProviderConfig>,
    config_path: Option<PathBuf>,
    config_watcher: Option<Box<dyn ConfigWatcher>>,
    retry_strategy: RetryStrategy,
    http_client: Option<reqwest::Client>,
    key_source: Option<std::sync::Arc<dyn crate::key_source::KeySource>>,
}

impl Default for ProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderBuilder {
    pub fn new() -> Self {
        Self {
            config: None,
            config_path: None,
            config_watcher: None,
            retry_strategy: RetryStrategy::default(),
            http_client: None,
            key_source: None,
        }
    }

    pub fn with_config(mut self, config: ProviderConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_path = Some(path.into());
        self
    }

    pub fn with_yaml_config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_path = Some(path.into());
        self
    }

    pub fn with_retry(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = strategy;
        self
    }

    pub fn with_config_watcher(mut self, watcher: impl ConfigWatcher + 'static) -> Self {
        self.config_watcher = Some(Box::new(watcher));
        self
    }

    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn with_key_source(mut self, key_source: std::sync::Arc<dyn crate::key_source::KeySource>) -> Self {
        self.key_source = Some(key_source);
        self
    }

    /// 构建 Provider 路由器 (v2 — 独立 API，不实现 LlmProvider)
    pub async fn build(self) -> Result<ProviderRouter, ProviderError> {
        let config = if let Some(cfg) = self.config {
            cfg
        } else if let Some(path) = self.config_path {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("json");
            match ext {
                "yaml" | "yml" => ProviderConfig::from_yaml_file(path).await?,
                _ => ProviderConfig::from_file(path).await?,
            }
        } else {
            return Err(ProviderError::Config(
                "必须提供 ProviderConfig 或配置文件路径".to_owned(),
            ));
        };

        let http_client = match self.http_client {
            Some(client) => client,
            None => reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .build()
                .map_err(|e| ProviderError::Config(format!("Failed to build HTTP client: {}", e)))?,
        };
        let mut providers: HashMap<String, Box<dyn LlmProvider>> = HashMap::new();

        for (name, def) in &config.providers {
            let provider: Box<dyn LlmProvider> = match def.provider_type {
                ProviderType::OpenAi => {
                    Box::new(OpenAiProvider::new(name.clone(), def, http_client.clone())?)
                }
                #[cfg(feature = "claude")]
                ProviderType::Claude => {
                    Box::new(ClaudeProvider::new(name.clone(), def, http_client.clone())?)
                }
                #[cfg(not(feature = "claude"))]
                ProviderType::Claude => {
                    return Err(ProviderError::Config(
                        "claude feature 未启用".to_owned(),
                    ))
                }
                ProviderType::Local => Box::new(LocalProvider::new(name.clone(), def)?),
            };
            providers.insert(name.clone(), provider);
        }

        let models = config.collect_models();
        let default_model = config
            .default_model
            .or_else(|| models.first().map(|m| m.name.clone()))
            .unwrap_or_default();

        Ok(ProviderRouter::new(
            providers,
            models,
            config.routing,
            default_model,
            self.key_source,
        ))
    }
}

use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Mutex;

use crate::error::EmbedError;
use crate::traits::{EmbedModelInfo, EmbedPricing, EmbeddingModel};

/// 测试用 Mock Embedder
#[derive(Debug)]
pub struct MockEmbedder {
    info: EmbedModelInfo,
    expected_input: Mutex<Option<Vec<String>>>,
    mock_output: Mutex<Vec<Vec<f32>>>,
    should_error: Mutex<Option<EmbedError>>,
}

impl MockEmbedder {
    /// 创建新的 MockEmbedder
    pub fn new(dimensions: usize, max_batch_size: usize) -> Self {
        Self {
            info: EmbedModelInfo {
                name: "mock-embedder".into(),
                display_name: "Mock Embedder".into(),
                supported_dimensions: None,
                current_dimension: dimensions,
                max_input_tokens: 1024,
                max_batch_size,
                pricing: EmbedPricing {
                    input_per_million: 0.0,
                },
            },
            expected_input: Mutex::new(None),
            mock_output: Mutex::new(vec![]),
            should_error: Mutex::new(None),
        }
    }

    /// 设置期望输入和返回输出
    pub fn expect_embed(&mut self, inputs: Vec<&str>, outputs: Vec<Vec<f32>>) -> &mut Self {
        *self.expected_input.get_mut().unwrap() = Some(inputs.iter().map(|s| s.to_string()).collect());
        *self.mock_output.get_mut().unwrap() = outputs;
        self
    }

    /// 设置应返回的错误
    pub fn set_error(&mut self, error: EmbedError) {
        *self.should_error.get_mut().unwrap() = Some(error);
    }

    /// 直接设置返回向量
    pub fn set_output(&mut self, vectors: Vec<Vec<f32>>) {
        *self.mock_output.get_mut().unwrap() = vectors;
    }
}

#[async_trait]
impl EmbeddingModel for MockEmbedder {
    async fn embed(&self, input: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if input.is_empty() {
            return Err(EmbedError::EmptyBatch);
        }

        // 检查是否设定了错误
        if let Some(ref err) = *self.should_error.lock().unwrap() {
            return Err(EmbedError::Model(format!("Mock error: {err}")));
        }

        // 检查期望输入
        if let Some(ref expected) = *self.expected_input.lock().unwrap() {
            let actual: Vec<String> = input.iter().map(|s| s.to_string()).collect();
            if &actual != expected {
                return Err(EmbedError::Model(format!(
                    "输入不匹配: expected {expected:?}, got {actual:?}"
                )));
            }
        }

        let output = self.mock_output.lock().unwrap();
        if !output.is_empty() {
            if output.len() != input.len() {
                return Err(EmbedError::Model(format!(
                    "输出数量不匹配: expected {}, got {}",
                    input.len(),
                    output.len()
                )));
            }
            return Ok(output.clone());
        }

        // 默认行为：每个输入生成零向量
        Ok(vec![vec![0.0; self.info.current_dimension]; input.len()])
    }

    fn model_info(&self) -> &EmbedModelInfo {
        &self.info
    }

    fn max_batch_size(&self) -> usize {
        self.info.max_batch_size
    }

    fn dimensions(&self) -> usize {
        self.info.current_dimension
    }
}

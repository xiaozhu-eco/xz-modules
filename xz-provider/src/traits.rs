use std::fmt::Debug;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::error::ProviderError;
use crate::types::{
    CompletionRequest, CompletionResponse, ModelInfo, RequestOptions, StreamEvent,
};

/// 统一的 LLM 服务提供者接口 (v2)
///
/// 设计原则：
/// - Tool calling 是一等公民，不是附加功能
/// - 流式返回事件枚举（文本、工具调用、思考过程、用量），不是纯文本增量
/// - 结构化输出是请求参数的 option，不是独立方法
/// - Token 计数和上下文窗口属于 `ModelInfo`，不属于 Provider
/// - 控制平面参数（超时、取消、元数据）通过 `RequestOptions` 传递，不混入请求数据
#[async_trait]
pub trait LlmProvider: Debug + Send + Sync {
    /// 核心补全方法。所有能力（tool calling、structured output、streaming）
    /// 通过请求参数中的 option 字段控制，不需要多个方法。
    ///
    /// `RequestOptions` 携带控制平面参数（超时、取消令牌、元数据），
    /// 与 `CompletionRequest`（数据平面）分离。
    async fn complete(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError>;

    /// 流式补全。返回 `StreamEvent` 枚举，覆盖文本增量、工具调用增量、
    /// 思考过程、用量统计等所有事件类型。
    ///
    /// 取消通过 `RequestOptions.cancel` (CancellationToken) 控制，
    /// 实现层使用 `stream.take_until(cancel.cancelled())` 零成本取消。
    async fn complete_stream(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
        ProviderError,
    >;

    /// 列出该 Provider 支持的模型及其能力声明。
    /// 路由层据此做能力感知路由。
    fn models(&self) -> &[ModelInfo];

    /// Provider 名称标识（用于日志和指标）。
    fn name(&self) -> &str;

    /// 模型名称（用于路由层默认选择）。
    fn default_model(&self) -> Option<&str> {
        self.models().first().map(|m| m.name.as_str())
    }
}

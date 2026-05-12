/// 取消令牌 —— 封装 `tokio_util::sync::CancellationToken`
///
/// 用于：
/// - 主动取消请求（用户终止、超时）
/// - 流式 `complete_stream` 中通过 `take_until` 零成本终止流
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: tokio_util::sync::CancellationToken,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    pub fn cancelled(&self) -> tokio_util::sync::CancellationToken {
        self.inner.clone()
    }

    pub fn child(&self) -> Self {
        Self {
            inner: self.inner.child_token(),
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl From<CancellationToken> for tokio_util::sync::CancellationToken {
    fn from(ct: CancellationToken) -> Self {
        ct.inner
    }
}

impl From<tokio_util::sync::CancellationToken> for CancellationToken {
    fn from(inner: tokio_util::sync::CancellationToken) -> Self {
        Self { inner }
    }
}

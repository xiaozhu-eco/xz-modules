use async_trait::async_trait;

use crate::error::XzTtsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTtsCredential {
    pub app_id: String,
    pub access_token: String,
    pub resource_id: String,
}

#[async_trait]
pub trait CredentialProvider: Send + Sync {
    /// Resolve TTS credentials. Called before each new connection.
    async fn resolve(&self) -> Result<ResolvedTtsCredential, XzTtsError>;
}

#[derive(Debug, Clone)]
pub struct StaticCredential {
    app_id: String,
    access_token: String,
    resource_id: String,
}

impl StaticCredential {
    pub fn new(
        app_id: impl Into<String>,
        access_token: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            access_token: access_token.into(),
            resource_id: resource_id.into(),
        }
    }
}

#[async_trait]
impl CredentialProvider for StaticCredential {
    async fn resolve(&self) -> Result<ResolvedTtsCredential, XzTtsError> {
        Ok(ResolvedTtsCredential {
            app_id: self.app_id.clone(),
            access_token: self.access_token.clone(),
            resource_id: self.resource_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn static_credential_resolve_returns_expected_values() {
        let credential = StaticCredential::new("app-123", "token-456", "res-789");

        let resolved = credential.resolve().await.expect("resolve should succeed");

        assert_eq!(resolved.app_id, "app-123");
        assert_eq!(resolved.access_token, "token-456");
        assert_eq!(resolved.resource_id, "res-789");
    }
}

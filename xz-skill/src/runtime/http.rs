#[cfg(feature = "http-tool")]
use std::collections::HashMap;

#[cfg(feature = "http-tool")]
use crate::error::SkillError;

#[cfg(feature = "http-tool")]
#[derive(Debug)]
pub struct HttpToolExecutor {
    client: reqwest::Client,
}

#[cfg(feature = "http-tool")]
impl HttpToolExecutor {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Execute an HTTP-based tool call. Returns the response body as a Value.
    pub async fn execute(
        &self,
        url: &str,
        method: &str,
        headers: &HashMap<String, String>,
        timeout_ms: u64,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, SkillError> {
        let req = self.build_request(url, method, headers, timeout_ms, args)?;
        let response = req
            .send()
            .await
            .map_err(|e| SkillError::Http(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| SkillError::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(SkillError::Http(format!(
                "HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }

        // Try to parse as JSON, fall back to plain text
        serde_json::from_str(&body).or_else(|_| {
            Ok(serde_json::Value::String(body))
        })
    }

    fn build_request(
        &self,
        url: &str,
        method: &str,
        headers: &HashMap<String, String>,
        timeout_ms: u64,
        args: &serde_json::Value,
    ) -> Result<reqwest::Request, SkillError> {
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let mut req_builder = match method.to_uppercase().as_str() {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url).json(args),
            "PUT" => self.client.put(url).json(args),
            "DELETE" => self.client.delete(url),
            _ => return Err(SkillError::Http(format!("Unsupported method: {}", method))),
        };

        req_builder = req_builder.timeout(timeout);

        for (key, value) in headers {
            req_builder = req_builder.header(key.as_str(), value.as_str());
        }

        if method.to_uppercase() == "GET" {
            req_builder = req_builder.query(&serde_json::json!({"q": args}));
        }

        req_builder.build().map_err(|e| SkillError::Http(e.to_string()))
    }
}

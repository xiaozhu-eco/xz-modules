use async_trait::async_trait;
use std::sync::Arc;

use crate::error::SkillError;
use crate::security::permissions::PermissionValidator;
use crate::traits::{SkillRegistry, SkillRuntime};
use crate::types::context::ExecutionContext;
use crate::types::filter::{PreflightWarning, WarningSeverity};
use crate::types::output::{SkillOutput, TokenUsage, ToolCallRecord};
use crate::types::skill::{Skill, ToolType};

/// Default skill runtime — LLM prompt injection + tool-calling loop.
#[derive(Debug)]
pub struct DefaultSkillRuntime {
    registry: Option<Arc<dyn SkillRegistry>>,
    permission_validator: PermissionValidator,
}

impl DefaultSkillRuntime {
    pub fn new(registry: Option<Arc<dyn SkillRegistry>>) -> Self {
        Self {
            registry,
            permission_validator: PermissionValidator::new(false, vec![]),
        }
    }

    pub fn with_permissions(mut self, allowed_network: bool, allowed_paths: Vec<std::path::PathBuf>) -> Self {
        self.permission_validator = PermissionValidator::new(allowed_network, allowed_paths);
        self
    }

    async fn resolve_skill(&self, skill_id: &str) -> Result<Skill, SkillError> {
        if let Some(ref reg) = self.registry {
            reg.get(skill_id)
                .await?
                .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))
        } else {
            Err(SkillError::ConfigValidation(
                "No skill registry configured".into(),
            ))
        }
    }
}

#[async_trait]
impl SkillRuntime for DefaultSkillRuntime {
    async fn execute(
        &self,
        skill_id: &str,
        input: &str,
        context: &ExecutionContext,
    ) -> Result<SkillOutput, SkillError> {
        let skill = self.resolve_skill(skill_id).await?;

        if !skill.enabled {
            return Err(SkillError::Disabled(skill_id.to_string()));
        }

        // Validate permissions
        self.validate_permissions(&skill, context).await?;

        // Validate version constraint
        if let Some(ref min_version) = skill.min_agent_version {
            // Simple version check: compare "0.1.0" format
            if let Err(e) = check_version(min_version, "0.1.0") {
                return Err(SkillError::VersionMismatch { required: e });
            }
        }

        let start = std::time::Instant::now();
        let mut tool_calls = Vec::new();

        // Simple tool execution loop — in production this would be LLM-driven
        for tool_def in &skill.tools {
            let tool_start = std::time::Instant::now();
            match &tool_def.tool_type {
                ToolType::Builtin { handler } => {
                    let result = execute_builtin_tool(handler, &serde_json::Value::String(input.to_string()))
                        .await;
                    tool_calls.push(ToolCallRecord {
                        tool_name: tool_def.name.clone(),
                        args: serde_json::json!({"input": input}),
                        result: result.as_ref().ok().cloned(),
                        error: result.as_ref().err().map(|e| e.to_string()),
                        duration_ms: tool_start.elapsed().as_millis() as u64,
                    });
                }
                #[cfg(feature = "http-tool")]
                ToolType::Http {
                    url,
                    method,
                    headers,
                    timeout_ms,
                } => {
                    let executor = crate::runtime::http::HttpToolExecutor::new();
                    let result = executor
                        .execute(
                            url,
                            method,
                            headers,
                            *timeout_ms,
                            &serde_json::json!({"input": input}),
                        )
                        .await;
                    tool_calls.push(ToolCallRecord {
                        tool_name: tool_def.name.clone(),
                        args: serde_json::json!({"input": input}),
                        result: result.as_ref().ok().cloned(),
                        error: result.as_ref().err().map(|e| e.to_string()),
                        duration_ms: tool_start.elapsed().as_millis() as u64,
                    });
                }
                #[cfg(feature = "wasm-runtime")]
                ToolType::Wasm {
                    module,
                    timeout_ms: _timeout_ms,
                    ..
                } => {
                    let runtime = crate::runtime::wasm::WasmRuntime::new(
                        crate::runtime::wasm::WasmConfig::default(),
                    )?;
                    let result = runtime
                        .execute(module, &tool_def.name, serde_json::json!({"input": input}))
                        .await;
                    tool_calls.push(ToolCallRecord {
                        tool_name: tool_def.name.clone(),
                        args: serde_json::json!({"input": input}),
                        result: result.as_ref().ok().cloned(),
                        error: result.as_ref().err().map(|e| e.to_string()),
                        duration_ms: tool_start.elapsed().as_millis() as u64,
                    });
                }
                _ => {
                    tool_calls.push(ToolCallRecord {
                        tool_name: tool_def.name.clone(),
                        args: serde_json::Value::Null,
                        result: None,
                        error: Some("Tool type not supported (missing feature flag)".into()),
                        duration_ms: 0,
                    });
                }
            }
        }

        let total_ms = start.elapsed().as_millis() as u64;

        Ok(SkillOutput {
            content: format!("Skill '{}' executed with {} tools", skill.name, tool_calls.len()),
            tool_calls,
            token_usage: TokenUsage::default(),
            duration_ms: total_ms,
        })
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, SkillError> {
        execute_builtin_tool(tool_name, &args).await
    }

    async fn validate_permissions(
        &self,
        skill: &Skill,
        _context: &ExecutionContext,
    ) -> Result<(), SkillError> {
        for perm in &skill.permissions {
            self.permission_validator.check(perm)?;
        }
        Ok(())
    }

    async fn preflight_check(&self, skill: &Skill) -> Result<Vec<PreflightWarning>, SkillError> {
        let mut warnings = Vec::new();

        // Check for missing prompt
        if skill.prompt.trim().is_empty() {
            warnings.push(PreflightWarning {
                severity: WarningSeverity::Error,
                message: "Skill has no prompt defined".into(),
            });
        }

        // Check WASM modules
        #[cfg(feature = "wasm-runtime")]
        for tool in &skill.tools {
            if let ToolType::Wasm {
                module, timeout_ms, ..
            } = &tool.tool_type
            {
                if module.is_empty() {
                    warnings.push(PreflightWarning {
                        severity: WarningSeverity::Error,
                        message: format!("WASM module is empty for tool '{}'", tool.name),
                    });
                }
                if *timeout_ms > 30_000 {
                    warnings.push(PreflightWarning {
                        severity: WarningSeverity::Warning,
                        message: format!(
                            "WASM tool '{}' timeout is > 30s ({}ms)",
                            tool.name, timeout_ms
                        ),
                    });
                }
            }
        }

        Ok(warnings)
    }
}

/// Simple built-in tool execution — placeholder for real implementations.
async fn execute_builtin_tool(
    handler: &str,
    args: &serde_json::Value,
) -> Result<serde_json::Value, SkillError> {
    match handler {
        "echo" => Ok(args.clone()),
        "now" => Ok(serde_json::json!({
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        })),
        "uuid" => {
            let id = uuid::Uuid::new_v4().to_string();
            Ok(serde_json::json!({"uuid": id}))
        }
        "json_path" => {
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let data = args.get("data").cloned().unwrap_or(serde_json::Value::Null);
            let result = extract_json_path(&data, path);
            Ok(result)
        }
        "base64_encode" => {
            let text = args.get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(serde_json::json!({
                "encoded": base64_encode(text)
            }))
        }
        "base64_decode" => {
            let text = args.get("encoded")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let decoded = base64_decode(text)
                .map_err(|e| SkillError::ToolExecution(e))?;
            Ok(serde_json::json!({
                "decoded": decoded
            }))
        }
        _ => Err(SkillError::ToolExecution(format!(
            "Unknown builtin handler: {}",
            handler
        ))),
    }
}

fn extract_json_path(data: &serde_json::Value, path: &str) -> serde_json::Value {
    if path.is_empty() || path == "$" {
        return data.clone();
    }
    let segments: Vec<&str> = path
        .trim_start_matches("$.")
        .split('.')
        .collect();
    let mut current = data;
    for seg in segments {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(seg).unwrap_or(&serde_json::Value::Null);
            }
            serde_json::Value::Array(arr) => {
                if let Ok(idx) = seg.parse::<usize>() {
                    current = arr.get(idx).unwrap_or(&serde_json::Value::Null);
                } else {
                    return serde_json::Value::Null;
                }
            }
            _ => return serde_json::Value::Null,
        }
    }
    current.clone()
}

fn base64_encode(text: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(text)
}

fn base64_decode(encoded: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|e| format!("Base64 decode failed: {}", e))?;
    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {}", e))
}

/// Simple semantic version check.
fn check_version(required: &str, actual: &str) -> Result<(), String> {
    let req = required.trim_start_matches(">=").trim();
    let req_parts: Vec<u32> = req.split('.').filter_map(|s| s.parse().ok()).collect();
    let act_parts: Vec<u32> = actual.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..req_parts.len().max(act_parts.len()) {
        let r = req_parts.get(i).copied().unwrap_or(0);
        let a = act_parts.get(i).copied().unwrap_or(0);
        if a < r {
            return Err(required.to_string());
        }
        if a > r {
            break;
        }
    }
    Ok(())
}

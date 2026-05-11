use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A registered skill with all its configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub prompt: String,
    pub tools: Vec<ToolDefinition>,
    pub config_schema: Option<serde_json::Value>,
    pub default_config: Option<serde_json::Value>,
    pub permissions: Vec<SkillPermission>,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub min_agent_version: Option<String>,
}

/// A tool available to a skill — may be built-in, WASM, or HTTP-based.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub tool_type: ToolType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolType {
    #[serde(rename = "builtin")]
    Builtin { handler: String },
    #[serde(rename = "wasm")]
    Wasm {
        #[serde(skip)]
        module: Vec<u8>,
        #[serde(default)]
        module_path: Option<String>,
        memory_limit_mb: u64,
        timeout_ms: u64,
    },
    #[serde(rename = "http")]
    Http {
        url: String,
        method: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        timeout_ms: u64,
    },
}

/// Permissions a skill may request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SkillPermission {
    Network,
    FileRead,
    FileWrite,
    Execute,
    Custom(String),
}

/// Result of an upsert (register) operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpsertResult {
    Created,
    Updated { changed_fields: Vec<String> },
    Unchanged,
}

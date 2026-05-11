use serde::{Deserialize, Serialize};

/// Message role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub role: Role,
    pub content: String,
    pub token_count: usize,
    pub created_at: u64,
    pub seq: u64,
}

impl Message {
    pub fn new(
        id: String,
        session_id: String,
        user_id: String,
        role: Role,
        content: String,
        token_count: usize,
    ) -> Self {
        Self {
            id,
            session_id,
            user_id,
            role,
            content,
            token_count,
            created_at: current_epoch_ms(),
            seq: 0,
        }
    }
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

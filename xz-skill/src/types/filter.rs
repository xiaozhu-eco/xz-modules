use serde::{Deserialize, Serialize};

/// Pagination request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    pub page: usize,
    pub page_size: usize,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self { page: 1, page_size: 20 }
    }
}

/// Filter for listing skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFilter {
    pub enabled_only: bool,
    pub author: Option<String>,
    pub page: PageRequest,
}

impl Default for SkillFilter {
    fn default() -> Self {
        Self {
            enabled_only: false,
            author: None,
            page: PageRequest::default(),
        }
    }
}

/// Preflight validation warning or error.
#[derive(Debug, Clone)]
pub struct PreflightWarning {
    pub severity: WarningSeverity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum WarningSeverity {
    Warning,
    Error,
}

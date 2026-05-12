use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 搜索配置 — 数据平面
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 最大返回结果数
    pub max_results: usize,
    /// 每个结果的最大 token 数
    pub max_tokens: usize,
    /// 指定搜索引擎源
    pub engines: Vec<String>,
    /// 搜索源类型
    pub sources: Vec<String>,
    /// 地区
    pub region: Option<String>,
    /// 语言偏好
    pub language: Option<String>,
    /// 时间范围过滤
    pub time_range: Option<TimeRange>,
    /// 是否使用缓存
    pub enable_cache: bool,
    /// 是否自动提取内容
    pub auto_extract: bool,
    /// 是否安全搜索
    pub safe_search: Option<SafeSearchLevel>,
    /// 分页偏移
    pub offset: usize,
    /// 是否执行查询重写
    pub enable_query_rewriting: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            max_tokens: 1024,
            engines: vec![],
            sources: vec!["web".into()],
            region: None,
            language: None,
            time_range: None,
            enable_cache: true,
            auto_extract: false,
            safe_search: Some(SafeSearchLevel::Moderate),
            offset: 0,
            enable_query_rewriting: false,
        }
    }
}

/// 搜索选项 — 控制平面
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// 请求超时
    pub timeout: Option<std::time::Duration>,
    /// 请求级元数据
    pub metadata: HashMap<String, String>,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

/// 安全搜索等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafeSearchLevel {
    Off,
    Moderate,
    Strict,
}

use serde::{Deserialize, Serialize};

/// 元数据过滤条件 — 支持组合查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetadataFilter {
    /// key = value
    Eq { key: String, value: String },
    /// key != value
    Ne { key: String, value: String },
    /// key IN (values)
    In { key: String, values: Vec<String> },
    /// key NOT IN (values)
    NotIn { key: String, values: Vec<String> },
    /// key 存在
    Exists { key: String },
    /// key 包含 value 子串
    Contains { key: String, value: String },
    /// 数值范围（f64）
    Range {
        key: String,
        min: Option<f64>,
        max: Option<f64>,
    },
    /// AND 组合
    And(Vec<MetadataFilter>),
    /// OR 组合
    Or(Vec<MetadataFilter>),
    /// NOT 取反
    Not(Box<MetadataFilter>),
}

impl MetadataFilter {
    pub fn eq(key: impl Into<String>, value: impl Into<String>) -> Self {
        MetadataFilter::Eq {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn ne(key: impl Into<String>, value: impl Into<String>) -> Self {
        MetadataFilter::Ne {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn in_values(key: impl Into<String>, values: &[impl AsRef<str>]) -> Self {
        MetadataFilter::In {
            key: key.into(),
            values: values.iter().map(|v| v.as_ref().to_string()).collect(),
        }
    }

    pub fn and(filters: impl IntoIterator<Item = MetadataFilter>) -> Self {
        MetadataFilter::And(filters.into_iter().collect())
    }

    pub fn or(filters: impl IntoIterator<Item = MetadataFilter>) -> Self {
        MetadataFilter::Or(filters.into_iter().collect())
    }

    pub fn not(filter: MetadataFilter) -> Self {
        MetadataFilter::Not(Box::new(filter))
    }
}

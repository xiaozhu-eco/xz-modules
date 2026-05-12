use serde::{Deserialize, Serialize};

/// 向量存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStats {
    pub total_vectors: usize,
    pub total_dimensions: usize,
    pub index_size_bytes: u64,
    pub data_size_bytes: u64,
    pub last_indexed_at: Option<u64>,
}

/// 缓存统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size_bytes: u64,
    pub entry_count: usize,
}

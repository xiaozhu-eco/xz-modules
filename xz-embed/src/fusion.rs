use std::collections::HashMap;

use crate::types::SearchResult;

/// 混合检索融合结果
#[derive(Debug, Clone)]
pub struct FusionResult {
    pub id: String,
    /// 融合后得分
    pub fused_score: f32,
    /// 向量相似度得分
    pub vector_score: f32,
    /// BM25 得分
    pub keyword_score: f32,
}

/// 倒数排名融合（Reciprocal Rank Fusion）
///
/// 将向量搜索结果和关键词搜索结果融合为统一的排序列表。
/// RRF 公式：score(d) = Σ 1/(k + rank_i(d))
pub fn rrf_fusion(
    vector_results: &[SearchResult],
    keyword_results: &[(String, f32)],
    k: f32,
) -> Vec<FusionResult> {
    // 分别按得分降序排名
    let mut vec_ranked: Vec<SearchResult> = vector_results.to_vec();
    vec_ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let mut kw_sorted: Vec<&(String, f32)> = keyword_results.iter().collect();
    kw_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 构建 RRF 得分表
    let mut rrf: HashMap<String, (f32, f32, f32)> = HashMap::new(); // id -> (vec_score, kw_score, fused)

    // 向量搜索结果贡献
    for (rank, result) in vec_ranked.iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);
        rrf.entry(result.id.clone())
            .and_modify(|(_, _, fused)| *fused += rrf_score)
            .or_insert_with(|| (result.score, 0.0, rrf_score));
    }

    // 关键词搜索结果贡献
    for (rank, (id, score)) in kw_sorted.iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f32 + 1.0);
        rrf.entry(id.clone())
            .and_modify(|(_, kw_s, fused)| {
                *kw_s = *score;
                *fused += rrf_score;
            })
            .or_insert_with(|| (0.0, *score, rrf_score));
    }

    let mut results: Vec<FusionResult> = rrf
        .into_iter()
        .map(|(id, (vector_score, keyword_score, fused_score))| FusionResult {
            id,
            fused_score,
            vector_score,
            keyword_score,
        })
        .collect();

    results.sort_by(|a, b| b.fused_score.partial_cmp(&a.fused_score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

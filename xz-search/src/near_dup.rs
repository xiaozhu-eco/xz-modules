use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 近重复检测器（基于 MinHash + LSH）
///
/// 用于检测不同 URL 指向相同内容的情况。
#[derive(Debug, Clone)]
pub struct NearDuplicateDetector {
    /// LSH 签名数（哈希函数数量）
    num_hashes: usize,
    /// 相似度阈值
    threshold: f32,
}

impl NearDuplicateDetector {
    pub fn new(num_hashes: usize, threshold: f32) -> Self {
        Self {
            num_hashes,
            threshold,
        }
    }

    /// 对内容计算 MinHash 签名（N-gram 哈希的最小值集合）
    pub fn compute_signature(&self, content: &str) -> Vec<u64> {
        let n = 3; // trigram
        let ngrams = self.get_ngrams(content, n);

        if ngrams.is_empty() {
            return vec![0; self.num_hashes];
        }

        let mut signatures = vec![u64::MAX; self.num_hashes];

        for ngram in &ngrams {
            let base_hash = self.hash_str(ngram);
            for i in 0..self.num_hashes {
                let h = self.rotated_hash(base_hash, i);
                if h < signatures[i] {
                    signatures[i] = h;
                }
            }
        }

        signatures
    }

    /// 检查两个签名是否可能为近重复
    pub fn is_near_duplicate(&self, sig_a: &[u64], sig_b: &[u64]) -> bool {
        if sig_a.len() != sig_b.len() {
            return false;
        }

        let mut matches = 0usize;
        for (a, b) in sig_a.iter().zip(sig_b) {
            if a == b {
                matches += 1;
            }
        }

        let similarity = matches as f32 / sig_a.len() as f32;
        similarity >= self.threshold
    }

    /// 从文本列表中去重（保留每组的第一条）
    pub fn deduplicate<T: Clone>(
        &self,
        items: Vec<(T, String)>,
    ) -> Vec<(T, String)> {
        let mut kept: Vec<(T, String, Vec<u64>)> = Vec::new();

        for (item, content) in items {
            let sig = self.compute_signature(&content);
            let mut is_dup = false;

            for (_, _, existing_sig) in &kept {
                if self.is_near_duplicate(&sig, existing_sig) {
                    is_dup = true;
                    break;
                }
            }

            if !is_dup {
                kept.push((item, content, sig));
            }
        }

        kept.into_iter().map(|(t, c, _)| (t, c)).collect()
    }

    fn get_ngrams(&self, content: &str, n: usize) -> Vec<String> {
        let chars: Vec<char> = content.chars().collect();
        if chars.len() < n {
            return vec![content.to_string()];
        }
        chars.windows(n).map(|w| w.iter().collect()).collect()
    }

    fn hash_str(&self, s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    fn rotated_hash(&self, base: u64, seed: usize) -> u64 {
        // 通过 seed 的位旋转生成不同哈希
        base.rotate_left(seed as u32)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15u64.wrapping_mul(seed as u64 + 1))
    }
}

impl Default for NearDuplicateDetector {
    fn default() -> Self {
        Self::new(128, 0.95)
    }
}

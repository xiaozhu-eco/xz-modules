use crate::quantize::VectorQuantizer;

/// 乘积量化（Product Quantization）
#[derive(Debug)]
pub struct ProductQuantizer {
    /// 子向量数
    num_sub_vectors: usize,
    /// 每个子向量的位数
    bits_per_sub_vector: usize,
    /// 训练好的码本: [sub_vec_idx][code_idx][dim]
    codebooks: Vec<Vec<Vec<f32>>>,
}

impl ProductQuantizer {
    /// 从样本训练 PQ
    pub fn train(
        samples: &[Vec<f32>],
        num_sub_vectors: usize,
        bits: usize,
    ) -> Result<Self, String> {
        if samples.is_empty() || num_sub_vectors == 0 {
            return Err("samples 和 num_sub_vectors 必须非空".into());
        }

        let dim = samples[0].len();
        let sub_dim = dim / num_sub_vectors;
        if sub_dim == 0 || dim % num_sub_vectors != 0 {
            return Err(format!("维度 {dim} 不能被 {num_sub_vectors} 整除"));
        }

        let num_clusters = 1usize << bits;
        let mut codebooks = Vec::with_capacity(num_sub_vectors);

        for s in 0..num_sub_vectors {
            // 提取子向量
            let start = s * sub_dim;
            let end = start + sub_dim;
            let sub_samples: Vec<Vec<f32>> = samples
                .iter()
                .map(|v| v[start..end].to_vec())
                .collect();

            // 简单 K-means 聚类（随机初始中心 + 迭代）
            let centroids = Self::kmeans_simple(&sub_samples, num_clusters, 10);
            codebooks.push(centroids);
        }

        Ok(Self {
            num_sub_vectors,
            bits_per_sub_vector: bits,
            codebooks,
        })
    }

    fn kmeans_simple(samples: &[Vec<f32>], k: usize, iterations: usize) -> Vec<Vec<f32>> {
        if samples.len() <= k {
            return samples.to_vec();
        }

        let dim = samples[0].len();
        // 随机选 k 个样本作为初始中心
        let mut centroids: Vec<Vec<f32>> = samples.iter().take(k).cloned().collect();
        let mut assignments = vec![0usize; samples.len()];

        for _ in 0..iterations {
            // 分配最近中心
            for (i, sample) in samples.iter().enumerate() {
                let mut min_dist = f32::MAX;
                let mut best = 0;
                for (j, centroid) in centroids.iter().enumerate() {
                    let dist: f32 = sample
                        .iter()
                        .zip(centroid)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum();
                    if dist < min_dist {
                        min_dist = dist;
                        best = j;
                    }
                }
                assignments[i] = best;
            }

            // 更新中心
            for j in 0..k {
                let members: Vec<&Vec<f32>> = samples
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| assignments[*i] == j)
                    .map(|(_, v)| v)
                    .collect();

                if !members.is_empty() {
                    let mut new_centroid = vec![0.0f32; dim];
                    for member in &members {
                        for d in 0..dim {
                            new_centroid[d] += member[d];
                        }
                    }
                    for d in 0..dim {
                        new_centroid[d] /= members.len() as f32;
                    }
                    centroids[j] = new_centroid;
                }
            }
        }

        centroids
    }

    /// 最近码字搜索
    fn nearest_codebook(&self, sub_vector: &[f32], codebook: &[Vec<f32>]) -> u8 {
        let mut min_dist = f32::MAX;
        let mut best = 0u8;
        for (i, centroid) in codebook.iter().enumerate() {
            let dist: f32 = sub_vector
                .iter()
                .zip(centroid)
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            if dist < min_dist {
                min_dist = dist;
                best = i as u8;
            }
        }
        best
    }
}

impl VectorQuantizer for ProductQuantizer {
    fn compress(&self, vectors: &[Vec<f32>]) -> Vec<Vec<u8>> {
        let dim = vectors.first().map(|v| v.len()).unwrap_or(0);
        let sub_dim = dim / self.num_sub_vectors;
        if sub_dim == 0 { return vec![]; }

        vectors.iter().map(|v| {
            let mut code = vec![0u8; self.num_sub_vectors];
            for s in 0..self.num_sub_vectors {
                let start = s * sub_dim;
                let end = start + sub_dim;
                code[s] = self.nearest_codebook(&v[start..end], &self.codebooks[s]);
            }
            code
        }).collect()
    }

    fn decompress(&self, quantized: &[Vec<u8>]) -> Vec<Vec<f32>> {
        let sub_dim = self.codebooks.first().map(|c| c.first().map(|v| v.len()).unwrap_or(0)).unwrap_or(0);
        let dim = self.num_sub_vectors * sub_dim;

        quantized.iter().map(|code| {
            let mut v = vec![0.0f32; dim];
            for (s, &c) in code.iter().enumerate().take(self.num_sub_vectors) {
                let start = s * sub_dim;
                let end = start + sub_dim;
                if let Some(centroid) = self.codebooks[s].get(c as usize) {
                    v[start..end].copy_from_slice(centroid);
                }
            }
            v
        }).collect()
    }
}

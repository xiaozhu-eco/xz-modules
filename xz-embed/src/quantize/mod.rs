pub mod product;
pub mod scalar;

pub use product::ProductQuantizer;
pub use scalar::ScalarQuantizer;

use std::fmt::Debug;

/// 向量量化 trait
pub trait VectorQuantizer: Send + Sync + Debug {
    /// 压缩向量（float → quantized）
    fn compress(&self, vectors: &[Vec<f32>]) -> Vec<Vec<u8>>;

    /// 解压向量（quantized → float）
    fn decompress(&self, quantized: &[Vec<u8>]) -> Vec<Vec<f32>>;
}

/// 空量化器（不做量化）
#[derive(Debug)]
pub struct NoopQuantizer;

impl VectorQuantizer for NoopQuantizer {
    fn compress(&self, vectors: &[Vec<f32>]) -> Vec<Vec<u8>> {
        vectors.iter().map(|v| v.iter().flat_map(|f| f.to_le_bytes()).collect()).collect()
    }

    fn decompress(&self, quantized: &[Vec<u8>]) -> Vec<Vec<f32>> {
        quantized.iter().map(|bytes| {
            bytes.chunks(4).map(|c| {
                let arr: [u8; 4] = c.try_into().unwrap_or([0; 4]);
                f32::from_le_bytes(arr)
            }).collect()
        }).collect()
    }
}

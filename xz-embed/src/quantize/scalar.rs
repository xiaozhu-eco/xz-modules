use crate::quantize::VectorQuantizer;

/// 标量量化（Scalar Quantization）
///
/// 将 float32 压缩到 u8（每维度 1 byte，压缩比 4:1）
#[derive(Debug)]
pub struct ScalarQuantizer {
    /// 每个维度的 min/max
    ranges: Vec<(f32, f32)>,
    /// 量化位数 (8 = u8)
    bits: usize,
}

impl ScalarQuantizer {
    pub fn new(ranges: Vec<(f32, f32)>, bits: usize) -> Self {
        Self { ranges, bits }
    }

    /// 从样本计算各维度的 min/max
    pub fn from_samples(samples: &[Vec<f32>], bits: usize) -> Self {
        if samples.is_empty() {
            return Self { ranges: vec![], bits };
        }

        let dim = samples[0].len();
        let mut ranges = Vec::with_capacity(dim);

        for d in 0..dim {
            let mut min = f32::MAX;
            let mut max = f32::MIN;
            for sample in samples {
                let val = sample[d];
                if val < min { min = val; }
                if val > max { max = val; }
            }
            ranges.push((min, max));
        }

        Self { ranges, bits }
    }

    fn quantize_value(&self, value: f32, min: f32, max: f32) -> u8 {
        let range = max - min;
        if range < f32::EPSILON {
            return 0;
        }
        let normalized = (value - min) / range;
        let max_val = (1u32 << self.bits) - 1;
        (normalized * max_val as f32).round().clamp(0.0, max_val as f32) as u8
    }

    fn dequantize_value(&self, q: u8, min: f32, max: f32) -> f32 {
        let max_val = (1u32 << self.bits) - 1;
        let normalized = q as f32 / max_val as f32;
        min + normalized * (max - min)
    }
}

impl VectorQuantizer for ScalarQuantizer {
    fn compress(&self, vectors: &[Vec<f32>]) -> Vec<Vec<u8>> {
        if self.ranges.is_empty() {
            return vec![];
        }

        vectors.iter().map(|v| {
            v.iter().enumerate().map(|(d, &val)| {
                let (min, max) = self.ranges[d];
                self.quantize_value(val, min, max)
            }).collect()
        }).collect()
    }

    fn decompress(&self, quantized: &[Vec<u8>]) -> Vec<Vec<f32>> {
        if self.ranges.is_empty() {
            return vec![];
        }

        quantized.iter().map(|q| {
            q.iter().enumerate().map(|(d, &qval)| {
                let (min, max) = self.ranges[d];
                self.dequantize_value(qval, min, max)
            }).collect()
        }).collect()
    }
}

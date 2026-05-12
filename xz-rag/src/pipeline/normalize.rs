/// Score normalization strategies.
///
/// Ensures scores from different channels are comparable before fusion.
#[derive(Debug, Clone)]
pub struct MinMaxNormalizer;

impl MinMaxNormalizer {
    /// Normalize scores to [0, 1] range using min-max scaling.
    pub fn normalize(scores: &mut [f32]) {
        if scores.is_empty() {
            return;
        }
        let min = scores.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;
        if range > 0.0 {
            for score in scores.iter_mut() {
                *score = (*score - min) / range;
            }
        } else {
            for score in scores.iter_mut() {
                *score = 0.5;
            }
        }
    }
}

/// Z-Score normalization.
#[derive(Debug, Clone)]
pub struct ZScoreNormalizer;

impl ZScoreNormalizer {
    pub fn normalize(scores: &mut [f32]) {
        if scores.is_empty() || scores.len() == 1 {
            return;
        }
        let mean = scores.iter().sum::<f32>() / scores.len() as f32;
        let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / scores.len() as f32;
        let std_dev = variance.sqrt();

        if std_dev > 0.0 {
            for score in scores.iter_mut() {
                *score = (*score - mean) / std_dev;
                // Clamp to [0, 1]
                *score = score.clamp(0.0, 1.0);
            }
        }
    }
}

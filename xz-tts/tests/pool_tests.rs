use tokio::sync::mpsc;
use xz_tts::pool::VolcengineTtsPool;
use xz_tts::StreamingTts;
use xz_tts::types::{AudioFormat, TtsSessionConfig, TtsVoiceInfo};

fn make_test_voices() -> Vec<TtsVoiceInfo> {
    vec![TtsVoiceInfo {
        voice_id: "test".into(),
        name: "Test".into(),
        gender: None,
        language: "zh".into(),
        styles: vec![],
        preview_url: None,
        scenarios: vec!["test".into()],
        model_version: "2.0".into(),
    }]
}

#[tokio::test]
async fn pool_creates_and_returns_voices() {
    let pool = VolcengineTtsPool::new(make_test_voices());
    assert_eq!(pool.available_voices().len(), 1);
}

#[tokio::test]
async fn pool_submit_returns_audio_receiver() {
    let pool = VolcengineTtsPool::new(make_test_voices());
    let (_tx, rx) = mpsc::channel::<String>(1);
    let config = TtsSessionConfig {
        format: AudioFormat {
            sample_rate: 24_000,
            channels: 1,
        },
        ..TtsSessionConfig::default()
    };
    let result = pool.submit(rx, config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn pool_shutdown_works() {
    let pool = VolcengineTtsPool::new(make_test_voices());
    pool.shutdown();
    let (_tx, rx) = mpsc::channel::<String>(1);
    let _ = pool.submit(rx, TtsSessionConfig::default());
}

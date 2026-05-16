use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::XzTtsError;
use crate::types::{AudioFrame, TtsSessionConfig, TtsVoiceInfo};

/// Streaming Text-to-Speech synthesis trait.
/// Uses mpsc channels for text input and audio output.
#[async_trait]
pub trait StreamingTts: Send + Sync {
    /// Synthesize text from a stream, producing audio frames.
    ///
    /// Default implementation delegates to `synthesize_streaming_with_config`
    /// with the default session config.
    async fn synthesize_streaming(
        &self,
        text_rx: mpsc::Receiver<String>,
    ) -> Result<mpsc::Receiver<Result<AudioFrame, XzTtsError>>, XzTtsError> {
        self.synthesize_streaming_with_config(text_rx, TtsSessionConfig::default())
            .await
    }

    /// Synthesize text from a stream with session configuration.
    async fn synthesize_streaming_with_config(
        &self,
        text_rx: mpsc::Receiver<String>,
        config: TtsSessionConfig,
    ) -> Result<mpsc::Receiver<Result<AudioFrame, XzTtsError>>, XzTtsError>;

    /// List available voices.
    fn available_voices(&self) -> &[TtsVoiceInfo];
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTts {
        voices: Vec<TtsVoiceInfo>,
    }

    #[async_trait]
    impl StreamingTts for MockTts {
        async fn synthesize_streaming_with_config(
            &self,
            _text_rx: mpsc::Receiver<String>,
            _config: TtsSessionConfig,
        ) -> Result<mpsc::Receiver<Result<AudioFrame, XzTtsError>>, XzTtsError> {
            let (tx, rx) = mpsc::channel(1);
            drop(tx);
            Ok(rx)
        }

        fn available_voices(&self) -> &[TtsVoiceInfo] {
            &self.voices
        }
    }

    #[tokio::test]
    async fn default_synthesize_delegates_to_with_config() {
        let tts = MockTts { voices: vec![] };
        let (_tx, rx) = mpsc::channel::<String>(1);

        let result = tts.synthesize_streaming(rx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn available_voices_returns_registry() {
        let voice = TtsVoiceInfo {
            voice_id: "test".into(),
            name: "Test".into(),
            gender: None,
            language: "zh".into(),
            styles: vec![],
            preview_url: None,
            scenarios: vec!["通用场景".into()],
            model_version: "2.0".into(),
        };

        let tts = MockTts { voices: vec![voice] };

        assert_eq!(tts.available_voices().len(), 1);
        assert_eq!(tts.available_voices()[0].voice_id, "test");
    }
}

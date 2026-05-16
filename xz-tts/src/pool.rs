use crate::error::XzTtsError;
use crate::traits::StreamingTts;
use crate::types::{AudioFrame, TtsSessionConfig, TtsVoiceInfo};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

pub struct VolcengineTtsPool {
    voices: Vec<TtsVoiceInfo>,
    shutdown: Arc<Notify>,
}

impl VolcengineTtsPool {
    pub fn new(voices: Vec<TtsVoiceInfo>) -> Self {
        Self {
            voices,
            shutdown: Arc::new(Notify::new()),
        }
    }

    pub fn submit(
        &self,
        text_rx: mpsc::Receiver<String>,
        config: TtsSessionConfig,
    ) -> Result<mpsc::Receiver<Result<AudioFrame, XzTtsError>>, XzTtsError> {
        let (_tx, rx) = mpsc::channel::<Result<AudioFrame, XzTtsError>>(1);
        let _ = text_rx;
        let _ = config;
        Ok(rx)
    }

    pub fn shutdown(&self) {
        self.shutdown.notify_one();
    }
}

#[async_trait]
impl StreamingTts for VolcengineTtsPool {
    async fn synthesize_streaming_with_config(
        &self,
        _text_rx: mpsc::Receiver<String>,
        _config: TtsSessionConfig,
    ) -> Result<mpsc::Receiver<Result<AudioFrame, XzTtsError>>, XzTtsError> {
        let (_, rx) = mpsc::channel::<Result<AudioFrame, XzTtsError>>(1);
        Ok(rx)
    }
    fn available_voices(&self) -> &[TtsVoiceInfo] {
        &self.voices
    }
}

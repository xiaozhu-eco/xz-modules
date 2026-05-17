use crate::client::VolcengineTtsClient;
use crate::credential::CredentialProvider;
use crate::error::XzTtsError;
use crate::preprocess::TextPreprocessor;
use crate::traits::StreamingTts;
use crate::types::{AudioFrame, TtsSessionConfig, TtsVoiceInfo};
use crate::voices::VoiceRegistry;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Notify, mpsc};

const POOL_QUEUE_SIZE: usize = 4;
const SESSION_AUDIO_BUFFER_SIZE: usize = 8;
const SESSION_RESULT_BUFFER_SIZE: usize = 8;

struct QueuedSession {
    text_rx: mpsc::Receiver<String>,
    config: TtsSessionConfig,
    result_tx: mpsc::Sender<Result<AudioFrame, XzTtsError>>,
}

pub struct VolcengineTtsPool {
    voice_registry: VoiceRegistry,
    session_tx: mpsc::Sender<QueuedSession>,
    shutdown: Arc<Notify>,
    closed: Arc<AtomicBool>,
}

impl VolcengineTtsPool {
    pub fn new(
        credential_provider: Box<dyn CredentialProvider>,
        voice_registry: VoiceRegistry,
        preprocessor: Box<dyn TextPreprocessor>,
        voice: &str,
        ws_url: &str,
        sample_rate: u32,
    ) -> Self {
        let voices = voice_registry.list_all().to_vec();
        let client = VolcengineTtsClient::new(
            credential_provider,
            VoiceRegistry::new().with_voices(voices),
            preprocessor,
            voice,
            ws_url,
            sample_rate,
        );
        let (session_tx, session_rx) = mpsc::channel(POOL_QUEUE_SIZE);
        let shutdown = Arc::new(Notify::new());
        let closed = Arc::new(AtomicBool::new(false));

        tokio::spawn(Self::run_worker(
            client,
            session_rx,
            Arc::clone(&shutdown),
            Arc::clone(&closed),
        ));

        Self {
            voice_registry,
            session_tx,
            shutdown,
            closed,
        }
    }

    pub fn submit(
        &self,
        text_rx: mpsc::Receiver<String>,
        config: TtsSessionConfig,
    ) -> Result<mpsc::Receiver<Result<AudioFrame, XzTtsError>>, XzTtsError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(XzTtsError::Internal {
                message: "tts pool is shut down".into(),
            });
        }

        let (result_tx, result_rx) = mpsc::channel(SESSION_RESULT_BUFFER_SIZE);
        self.session_tx
            .try_send(QueuedSession {
                text_rx,
                config,
                result_tx,
            })
            .map_err(|err| XzTtsError::Internal {
                message: match err {
                    mpsc::error::TrySendError::Full(_) => "session queue full".into(),
                    mpsc::error::TrySendError::Closed(_) => "tts pool worker is unavailable".into(),
                },
            })?;

        Ok(result_rx)
    }

    pub fn shutdown(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.shutdown.notify_waiters();
        }
    }

    async fn run_worker(
        client: VolcengineTtsClient,
        mut session_rx: mpsc::Receiver<QueuedSession>,
        shutdown: Arc<Notify>,
        closed: Arc<AtomicBool>,
    ) {
        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                session = session_rx.recv() => {
                    match session {
                        Some(session) => {
                            if !Self::drain_session(&client, session, shutdown.as_ref()).await {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        closed.store(true, Ordering::SeqCst);
        client.shutdown();
        session_rx.close();
        while session_rx.recv().await.is_some() {}
    }

    async fn drain_session(
        client: &VolcengineTtsClient,
        session: QueuedSession,
        shutdown: &Notify,
    ) -> bool {
        let (audio_tx, mut audio_rx) = mpsc::channel(SESSION_AUDIO_BUFFER_SIZE);

        if let Err(err) = client.submit_session(session.text_rx, audio_tx, session.config) {
            let _ = session.result_tx.send(Err(err)).await;
            return true;
        }

        while let Some(frame) = tokio::select! {
            _ = shutdown.notified() => {
                client.shutdown();
                return false;
            }
            frame = audio_rx.recv() => frame,
        } {
            let _ = session.result_tx.send(Ok(frame)).await;
        }

        true
    }
}

impl Drop for VolcengineTtsPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[async_trait]
impl StreamingTts for VolcengineTtsPool {
    async fn synthesize_streaming_with_config(
        &self,
        text_rx: mpsc::Receiver<String>,
        config: TtsSessionConfig,
    ) -> Result<mpsc::Receiver<Result<AudioFrame, XzTtsError>>, XzTtsError> {
        self.submit(text_rx, config)
    }

    fn available_voices(&self) -> &[TtsVoiceInfo] {
        self.voice_registry.list_all()
    }
}

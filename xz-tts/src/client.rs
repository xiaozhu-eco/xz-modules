use crate::credential::{CredentialProvider, ResolvedTtsCredential};
use crate::error::XzTtsError;
use crate::preprocess::TextPreprocessor;
use crate::protocol::{
    EVT_CANCEL_SESSION, EVT_FINISH_CONNECTION, EVT_FINISH_SESSION, EVT_START_CONNECTION,
    EVT_START_SESSION, EVT_TASK_REQUEST, ServerEvent, build_connection_frame,
    build_session_frame, parse_server_frame,
};
use crate::types::{AudioFormat, AudioFrame, TtsSessionConfig};
use crate::voices::VoiceRegistry;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Notify, mpsc};
use tokio::time::{Instant, sleep};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const PCM_I16_SCALE: f32 = 32768.0;
const MAX_RECONNECT_ATTEMPTS: u32 = 20;
const RECONNECT_BASE_DELAY_MS: u64 = 1000;
const RECONNECT_MAX_DELAY_MS: u64 = 30000;
const SESSION_QUEUE_SIZE: usize = 4;

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    WsMessage,
>;

type WsStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

pub struct VolcengineTtsClient {
    session_tx: mpsc::Sender<TtsSession>,
    shutdown: Arc<Notify>,
}

struct TtsSession {
    text_rx: mpsc::Receiver<String>,
    audio_tx: mpsc::Sender<AudioFrame>,
    config: TtsSessionConfig,
}

struct ActiveSession {
    session_id: String,
    text_rx: mpsc::Receiver<String>,
    audio_tx: mpsc::Sender<AudioFrame>,
    audio_format: AudioFormat,
    started: bool,
    text_done: bool,
}

enum DisconnectReason {
    Shutdown,
    ChannelClosed,
    ServerClosed,
    Error(XzTtsError),
}

enum SessionAction {
    Continue,
    Completed,
    ReceiverClosed,
}

enum WsMsgResult {
    Binary(Vec<u8>),
    Disconnected(DisconnectReason),
    Ignored,
}

impl VolcengineTtsClient {
    pub fn new(
        credential_provider: Box<dyn CredentialProvider>,
        voice_registry: VoiceRegistry,
        preprocessor: Box<dyn TextPreprocessor>,
        voice: &str,
        ws_url: &str,
        sample_rate: u32,
    ) -> Self {
        let (session_tx, session_rx) = mpsc::channel::<TtsSession>(SESSION_QUEUE_SIZE);
        let shutdown = Arc::new(Notify::new());

        let voice = voice.to_string();
        let ws_url = ws_url.to_string();
        let shutdown_clone = Arc::clone(&shutdown);

        tokio::spawn(async move {
            Self::connection_manager(
                credential_provider,
                voice_registry,
                preprocessor,
                voice,
                ws_url,
                sample_rate,
                session_rx,
                shutdown_clone,
            )
            .await;
        });

        Self {
            session_tx,
            shutdown,
        }
    }

    pub fn submit_session(
        &self,
        text_rx: mpsc::Receiver<String>,
        audio_tx: mpsc::Sender<AudioFrame>,
        config: TtsSessionConfig,
    ) -> Result<(), XzTtsError> {
        self.session_tx
            .try_send(TtsSession {
                text_rx,
                audio_tx,
                config,
            })
            .map_err(|e| XzTtsError::Internal {
                message: format!("session channel full: {e}"),
            })
    }

    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
    }

    async fn connection_manager(
        credential_provider: Box<dyn CredentialProvider>,
        voice_registry: VoiceRegistry,
        preprocessor: Box<dyn TextPreprocessor>,
        voice: String,
        ws_url: String,
        sample_rate: u32,
        mut session_rx: mpsc::Receiver<TtsSession>,
        shutdown: Arc<Notify>,
    ) {
        let mut reconnect_attempts = 0_u32;

        loop {
            info!(
                "[volcengine-tts] connecting websocket (attempt #{})",
                reconnect_attempts + 1
            );

            let credential = match credential_provider.resolve().await {
                Ok(credential) => credential,
                Err(err) => {
                    error!("[volcengine-tts] credential resolution failed: {err}");
                    if Self::wait_before_retry(&shutdown, &mut reconnect_attempts).await {
                        return;
                    }
                    continue;
                }
            };

            match Self::connect_and_init(&credential, &ws_url).await {
                Ok((write, read)) => {
                    info!("[volcengine-tts] websocket connected and initialized");
                    reconnect_attempts = 0;

                    match Self::work_loop(
                        write,
                        read,
                        &voice_registry,
                        &*preprocessor,
                        &voice,
                        sample_rate,
                        &mut session_rx,
                        &shutdown,
                    )
                    .await
                    {
                        DisconnectReason::Shutdown | DisconnectReason::ChannelClosed => return,
                        DisconnectReason::ServerClosed => {
                            warn!("[volcengine-tts] server closed connection; reconnecting");
                        }
                        DisconnectReason::Error(err) => {
                            warn!("[volcengine-tts] connection loop ended: {err}");
                        }
                    }
                }
                Err(XzTtsError::Auth { message }) => {
                    error!("[volcengine-tts] fatal auth/request error: {message}");
                    return;
                }
                Err(err) => {
                    error!("[volcengine-tts] connection failed: {err}");
                }
            }

            if Self::wait_before_retry(&shutdown, &mut reconnect_attempts).await {
                return;
            }
        }
    }

    async fn wait_before_retry(shutdown: &Arc<Notify>, reconnect_attempts: &mut u32) -> bool {
        *reconnect_attempts += 1;
        if *reconnect_attempts > MAX_RECONNECT_ATTEMPTS {
            error!(
                "[volcengine-tts] reconnect failed {} times; giving up",
                MAX_RECONNECT_ATTEMPTS
            );
            return true;
        }

        let delay = Self::calc_reconnect_delay(*reconnect_attempts);
        info!(
            "[volcengine-tts] retrying in {:.1}s",
            delay.as_secs_f32()
        );

        tokio::select! {
            _ = sleep(delay) => false,
            _ = shutdown.notified() => true,
        }
    }

    async fn connect_and_init(
        credential: &ResolvedTtsCredential,
        ws_url: &str,
    ) -> Result<(WsSink, WsStream), XzTtsError> {
        let connect_id = uuid::Uuid::new_v4().to_string();
        let masked_token = mask_secret(&credential.access_token);

        info!(
            "[volcengine-tts] connect app_id={}, token={}, resource_id={}, connect_id={}",
            credential.app_id, masked_token, credential.resource_id, connect_id
        );

        let mut request = ws_url.into_client_request().map_err(|e| XzTtsError::Config {
            message: format!("invalid websocket url: {e}"),
        })?;

        {
            let headers = request.headers_mut();
            headers.insert(
                "X-Api-App-Key",
                credential.app_id.parse().map_err(|e| XzTtsError::Config {
                    message: format!("invalid app_id header: {e}"),
                })?,
            );
            headers.insert(
                "X-Api-Access-Key",
                credential
                    .access_token
                    .parse()
                    .map_err(|e| XzTtsError::Config {
                        message: format!("invalid access_token header: {e}"),
                    })?,
            );
            headers.insert(
                "X-Api-Resource-Id",
                credential
                    .resource_id
                    .parse()
                    .map_err(|e| XzTtsError::Config {
                        message: format!("invalid resource_id header: {e}"),
                    })?,
            );
            headers.insert(
                "X-Api-Connect-Id",
                connect_id.parse().map_err(|e| XzTtsError::Config {
                    message: format!("invalid connect_id header: {e}"),
                })?,
            );
        }

        let (ws, response) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(map_connect_error)?;

        if let Some(logid) = response.headers().get("X-Tt-Logid") {
            debug!("[volcengine-tts] X-Tt-Logid: {:?}", logid);
        }

        let (mut write, mut read) = ws.split();
        write
            .send(WsMessage::Binary(
                build_connection_frame(EVT_START_CONNECTION).into(),
            ))
            .await
            .map_err(|e| XzTtsError::Network {
                message: format!("failed to send StartConnection: {e}"),
            })?;

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(WsMessage::Binary(data))) => match parse_server_frame(&data) {
                            Ok(ServerEvent::ConnectionStarted) => return Ok((write, read)),
                            Ok(ServerEvent::ConnectionFailed { message }) => {
                                return Err(XzTtsError::Auth { message });
                            }
                            Ok(ServerEvent::ProtocolError { code, message }) => {
                                return Err(XzTtsError::Protocol { code, message });
                            }
                            Ok(other) => {
                                debug!(
                                    "[volcengine-tts] ignoring {:?} while waiting for ConnectionStarted",
                                    other
                                );
                            }
                            Err(err) => warn!("[volcengine-tts] failed to parse handshake frame: {err}"),
                        },
                        Some(Ok(WsMessage::Ping(payload))) => {
                            let _ = write.send(WsMessage::Pong(payload)).await;
                        }
                        Some(Ok(WsMessage::Text(text))) => {
                            warn!("[volcengine-tts] unexpected text during handshake: {text}");
                        }
                        Some(Ok(WsMessage::Close(_))) | None => {
                            return Err(XzTtsError::Network {
                                message: "connection closed during handshake".into(),
                            });
                        }
                        Some(Err(err)) => {
                            return Err(XzTtsError::Network {
                                message: format!("websocket read error during handshake: {err}"),
                            });
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(XzTtsError::Timeout {
                        message: "timed out waiting for ConnectionStarted".into(),
                    });
                }
            }
        }
    }

    async fn work_loop(
        mut write: WsSink,
        mut read: WsStream,
        voice_registry: &VoiceRegistry,
        preprocessor: &dyn TextPreprocessor,
        default_voice: &str,
        default_sample_rate: u32,
        session_rx: &mut mpsc::Receiver<TtsSession>,
        shutdown: &Arc<Notify>,
    ) -> DisconnectReason {
        let mut active_session: Option<ActiveSession> = None;

        loop {
            if let Some(session) = active_session.as_mut() {
                if !session.started {
                    tokio::select! {
                        msg = read.next() => {
                            match Self::handle_ws_msg(msg, &mut write).await {
                                WsMsgResult::Binary(data) => match parse_server_frame(&data) {
                                    Ok(ServerEvent::SessionStarted) => {
                                        info!("[volcengine-tts] session started: {}", session.session_id);
                                        session.started = true;
                                    }
                                    Ok(ServerEvent::SessionFailed { message }) => {
                                        warn!("[volcengine-tts] session start failed: {message}");
                                        active_session = None;
                                    }
                                    Ok(ServerEvent::ProtocolError { code, message }) => {
                                        return DisconnectReason::Error(XzTtsError::Protocol { code, message });
                                    }
                                    Ok(other) => debug!(
                                        "[volcengine-tts] ignoring {:?} while waiting for SessionStarted",
                                        other
                                    ),
                                    Err(err) => warn!("[volcengine-tts] failed to parse frame: {err}"),
                                },
                                WsMsgResult::Disconnected(reason) => return reason,
                                WsMsgResult::Ignored => {}
                            }
                        }
                        _ = shutdown.notified() => {
                            return Self::shutdown_connection(&mut write, &mut read, active_session.take()).await;
                        }
                    }
                } else {
                    let mut session_completed = false;

                    tokio::select! {
                        msg = read.next() => {
                            match Self::handle_ws_msg(msg, &mut write).await {
                                WsMsgResult::Binary(data) => match parse_server_frame(&data) {
                                    Ok(event) => match Self::handle_session_event(event, &session.audio_tx, session.audio_format).await {
                                        Ok(SessionAction::Continue) => {}
                                        Ok(SessionAction::Completed) => {
                                            info!("[volcengine-tts] session completed: {}", session.session_id);
                                            session_completed = true;
                                        }
                                        Ok(SessionAction::ReceiverClosed) => {
                                            warn!("[volcengine-tts] audio receiver closed; canceling session {}", session.session_id);
                                            if let Err(err) = Self::cancel_session(&mut write, &session.session_id).await {
                                                return DisconnectReason::Error(err);
                                            }
                                            match Self::drain_until_session_end(&mut read, &mut write, shutdown).await {
                                                Ok(()) => {
                                                    active_session = None;
                                                    continue;
                                                }
                                                Err(reason) => return reason,
                                            }
                                        }
                                        Err(err) => warn!("[volcengine-tts] failed to handle session event: {err}"),
                                    },
                                    Err(err) => warn!("[volcengine-tts] failed to parse frame: {err}"),
                                },
                                WsMsgResult::Disconnected(reason) => return reason,
                                WsMsgResult::Ignored => {}
                            }
                        }
                        text = session.text_rx.recv(), if !session.text_done => {
                            match text {
                                Some(text) if !text.trim().is_empty() => {
                                    let processed = match preprocessor.preprocess(&text).await {
                                        Ok(processed) => processed,
                                        Err(err) => return DisconnectReason::Error(err),
                                    };

                                    if !processed.trim().is_empty() {
                                        let payload = json!({
                                            "event": EVT_TASK_REQUEST,
                                            "req_params": { "text": processed }
                                        });
                                        let frame = build_session_frame(
                                            EVT_TASK_REQUEST,
                                            &session.session_id,
                                            payload.to_string().as_bytes(),
                                        );
                                        if let Err(err) = write.send(WsMessage::Binary(frame.into())).await {
                                            return DisconnectReason::Error(XzTtsError::Network {
                                                message: format!("failed to send TaskRequest: {err}"),
                                            });
                                        }
                                    }
                                }
                                Some(_) => {}
                                None => {
                                    let frame = build_session_frame(EVT_FINISH_SESSION, &session.session_id, b"{}");
                                    if let Err(err) = write.send(WsMessage::Binary(frame.into())).await {
                                        return DisconnectReason::Error(XzTtsError::Network {
                                            message: format!("failed to send FinishSession: {err}"),
                                        });
                                    }
                                    session.text_done = true;
                                }
                            }
                        }
                        _ = shutdown.notified() => {
                            return Self::shutdown_connection(&mut write, &mut read, active_session.take()).await;
                        }
                    }

                    if session_completed {
                        active_session = None;
                    }
                }
            } else {
                tokio::select! {
                    msg = read.next() => {
                        match Self::handle_ws_msg(msg, &mut write).await {
                            WsMsgResult::Binary(data) => match parse_server_frame(&data) {
                                Ok(ServerEvent::ProtocolError { code, message }) => {
                                    return DisconnectReason::Error(XzTtsError::Protocol { code, message });
                                }
                                Ok(ServerEvent::ConnectionFailed { message }) => {
                                    return DisconnectReason::Error(XzTtsError::Auth { message });
                                }
                                Ok(ServerEvent::SessionFailed { message }) => {
                                    warn!("[volcengine-tts] idle session failure event: {message}");
                                }
                                Ok(other) => debug!("[volcengine-tts] idle event: {:?}", other),
                                Err(err) => warn!("[volcengine-tts] failed to parse idle frame: {err}"),
                            },
                            WsMsgResult::Disconnected(reason) => return reason,
                            WsMsgResult::Ignored => {}
                        }
                    }
                    session = session_rx.recv() => {
                        match session {
                            Some(session) => {
                                let mut config = session.config.clone();
                                let selected_voice = resolve_voice_id(voice_registry, default_voice, &config.voice_id);
                                config.voice_id = selected_voice.clone();
                                if config.sample_rate == 0 {
                                    config.sample_rate = default_sample_rate;
                                }
                                if config.format.sample_rate == 0 {
                                    config.format.sample_rate = config.sample_rate;
                                }
                                if config.format.channels == 0 {
                                    config.format.channels = 1;
                                }

                                let session_id = uuid::Uuid::new_v4().to_string();
                                let payload = build_start_session_payload(&selected_voice, &config);
                                let frame = build_session_frame(
                                    EVT_START_SESSION,
                                    &session_id,
                                    payload.to_string().as_bytes(),
                                );

                                if let Err(err) = write.send(WsMessage::Binary(frame.into())).await {
                                    return DisconnectReason::Error(XzTtsError::Network {
                                        message: format!("failed to send StartSession: {err}"),
                                    });
                                }

                                active_session = Some(ActiveSession {
                                    session_id,
                                    text_rx: session.text_rx,
                                    audio_tx: session.audio_tx,
                                    audio_format: config.format,
                                    started: false,
                                    text_done: false,
                                });
                            }
                            None => {
                                let _ = write.send(WsMessage::Binary(build_connection_frame(EVT_FINISH_CONNECTION).into())).await;
                                let _ = write.send(WsMessage::Close(None)).await;
                                return DisconnectReason::ChannelClosed;
                            }
                        }
                    }
                    _ = shutdown.notified() => {
                        return Self::shutdown_connection(&mut write, &mut read, None).await;
                    }
                }
            }
        }
    }

    async fn shutdown_connection(
        write: &mut WsSink,
        read: &mut WsStream,
        active_session: Option<ActiveSession>,
    ) -> DisconnectReason {
        if let Some(session) = active_session {
            let _ = Self::cancel_session(write, &session.session_id).await;
            let _ = Self::drain_until_session_end(read, write, &Arc::new(Notify::new())).await;
        }

        let _ = write
            .send(WsMessage::Binary(
                build_connection_frame(EVT_FINISH_CONNECTION).into(),
            ))
            .await;
        let _ = write.send(WsMessage::Close(None)).await;
        DisconnectReason::Shutdown
    }

    async fn cancel_session(write: &mut WsSink, session_id: &str) -> Result<(), XzTtsError> {
        let frame = build_session_frame(EVT_CANCEL_SESSION, session_id, b"{}");
        write
            .send(WsMessage::Binary(frame.into()))
            .await
            .map_err(|err| XzTtsError::Network {
                message: format!("failed to send CancelSession: {err}"),
            })
    }

    async fn handle_ws_msg(
        msg: Option<Result<WsMessage, tokio_tungstenite::tungstenite::Error>>,
        write: &mut WsSink,
    ) -> WsMsgResult {
        match msg {
            Some(Ok(WsMessage::Binary(data))) => WsMsgResult::Binary(data.to_vec()),
            Some(Ok(WsMessage::Ping(data))) => {
                let _ = write.send(WsMessage::Pong(data)).await;
                WsMsgResult::Ignored
            }
            Some(Ok(WsMessage::Pong(_))) => WsMsgResult::Ignored,
            Some(Ok(WsMessage::Close(_))) => WsMsgResult::Disconnected(DisconnectReason::ServerClosed),
            Some(Ok(WsMessage::Text(text))) => {
                warn!("[volcengine-tts] received unexpected text frame: {text}");
                WsMsgResult::Ignored
            }
            Some(Err(err)) => WsMsgResult::Disconnected(DisconnectReason::Error(XzTtsError::Network {
                message: format!("websocket read error: {err}"),
            })),
            None => WsMsgResult::Disconnected(DisconnectReason::ServerClosed),
            _ => WsMsgResult::Ignored,
        }
    }

    async fn handle_session_event(
        event: ServerEvent,
        audio_tx: &mpsc::Sender<AudioFrame>,
        format: AudioFormat,
    ) -> Result<SessionAction, XzTtsError> {
        match event {
            ServerEvent::TtsAudioData { audio } => {
                let samples = audio
                    .chunks_exact(2)
                    .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / PCM_I16_SCALE)
                    .collect::<Vec<_>>();

                let frame = AudioFrame {
                    samples,
                    format,
                    timestamp_ms: 0,
                };

                if audio_tx.send(frame).await.is_err() {
                    return Ok(SessionAction::ReceiverClosed);
                }
                Ok(SessionAction::Continue)
            }
            ServerEvent::SessionFinished => Ok(SessionAction::Completed),
            ServerEvent::SessionFailed { message } => {
                warn!("[volcengine-tts] session failed: {message}");
                Ok(SessionAction::Completed)
            }
            ServerEvent::SessionCanceled => Ok(SessionAction::Completed),
            ServerEvent::TtsSentenceStart | ServerEvent::TtsSentenceEnd => Ok(SessionAction::Continue),
            ServerEvent::ProtocolError { code, message } => {
                warn!("[volcengine-tts] protocol error {code}: {message}");
                Ok(SessionAction::Completed)
            }
            _ => Ok(SessionAction::Continue),
        }
    }

    async fn drain_until_session_end(
        read: &mut WsStream,
        write: &mut WsSink,
        shutdown: &Arc<Notify>,
    ) -> Result<(), DisconnectReason> {
        let deadline = Instant::now() + Duration::from_secs(2);

        loop {
            tokio::select! {
                msg = read.next() => {
                    match Self::handle_ws_msg(msg, write).await {
                        WsMsgResult::Binary(data) => {
                            if let Ok(event) = parse_server_frame(&data) {
                                if matches!(
                                    event,
                                    ServerEvent::SessionFinished
                                        | ServerEvent::SessionCanceled
                                        | ServerEvent::SessionFailed { .. }
                                ) {
                                    return Ok(());
                                }
                            }
                        }
                        WsMsgResult::Disconnected(reason) => return Err(reason),
                        WsMsgResult::Ignored => {}
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    warn!("[volcengine-tts] session drain timed out after 2s");
                    return Ok(());
                }
                _ = shutdown.notified() => {
                    let _ = write.send(WsMessage::Close(None)).await;
                    return Err(DisconnectReason::Shutdown);
                }
            }
        }
    }

    fn calc_reconnect_delay(attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(10);
        let multiplier = 1_u64.checked_shl(shift).unwrap_or(u64::MAX);
        let base_ms = RECONNECT_BASE_DELAY_MS
            .saturating_mul(multiplier)
            .min(RECONNECT_MAX_DELAY_MS);

        let jitter_window = (base_ms / 4).max(1);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let jitter = nanos % jitter_window;
        Duration::from_millis(base_ms.saturating_sub(jitter_window / 2) + jitter)
    }
}

impl Drop for VolcengineTtsClient {
    fn drop(&mut self) {
        self.shutdown.notify_waiters();
    }
}

fn build_start_session_payload(voice: &str, config: &TtsSessionConfig) -> serde_json::Value {
    let mut audio_params = json!({
        "format": "pcm",
        "sample_rate": config.sample_rate,
    });
    if let Some(ref emotion) = config.emotion_tag {
        if !emotion.is_empty() {
            audio_params["emotion"] = json!(emotion);
        }
    }
    if let Some(speech_rate) = config.speech_rate {
        audio_params["speech_rate"] = json!(speech_rate);
    }
    if let Some(loudness_rate) = config.loudness_rate {
        audio_params["loudness_rate"] = json!(loudness_rate);
    }

    let mut additions = json!({
        "disable_markdown_filter": config.disable_markdown_filter
    });
    if let Some(pitch) = config.pitch {
        additions["post_process"] = json!({ "pitch": pitch });
    }

    json!({
        "event": EVT_START_SESSION,
        "namespace": "BidirectionalTTS",
        "req_params": {
            "speaker": voice,
            "audio_params": audio_params,
            "additions": additions.to_string(),
        }
    })
}

fn resolve_voice_id(voice_registry: &VoiceRegistry, default_voice: &str, requested_voice: &str) -> String {
    let candidate = if requested_voice.trim().is_empty() {
        default_voice
    } else {
        requested_voice.trim()
    };

    if let Some(voice) = voice_registry.get(candidate) {
        return voice.voice_id.clone();
    }

    if let Some(voice) = voice_registry.get(default_voice) {
        warn!(
            "[volcengine-tts] unknown voice '{}'; falling back to '{}'",
            candidate, default_voice
        );
        return voice.voice_id.clone();
    }

    candidate.to_string()
}

fn mask_secret(secret: &str) -> String {
    if secret.len() > 6 {
        format!("{}***{}", &secret[..3], &secret[secret.len() - 3..])
    } else {
        "***".to_string()
    }
}

fn map_connect_error(err: tokio_tungstenite::tungstenite::Error) -> XzTtsError {
    use tokio_tungstenite::tungstenite::Error as WsError;

    match err {
        WsError::Http(response) => {
            let status = response.status();
            let body = response
                .body()
                .as_ref()
                .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                .unwrap_or_else(|| "(empty response body)".to_string());
            let message = format!("HTTP {}: {}", status.as_u16(), body);
            if status.is_client_error() {
                XzTtsError::Auth { message }
            } else {
                XzTtsError::Network { message }
            }
        }
        other => XzTtsError::Network {
            message: format!("websocket connect failed: {other}"),
        },
    }
}

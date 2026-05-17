use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};
use xz_tts::credential::StaticCredential;
use xz_tts::pool::VolcengineTtsPool;
use xz_tts::preprocess::NoOpPreprocessor;
use xz_tts::protocol::{
    EVT_CONNECTION_STARTED, EVT_FINISH_CONNECTION, EVT_FINISH_SESSION, EVT_SESSION_FINISHED,
    EVT_SESSION_STARTED, EVT_START_CONNECTION, EVT_START_SESSION, EVT_TTS_RESPONSE,
    FLAG_WITH_EVENT, MSG_AUDIO_ONLY, MSG_FULL_SERVER_RESP, PROTO_HEADER, SERIAL_JSON, SERIAL_RAW,
};
use xz_tts::types::{AudioFormat, AudioFrame, TtsOutputFormat, TtsSessionConfig, TtsVoiceInfo};
use xz_tts::voices::VoiceRegistry;
use xz_tts::StreamingTts;

fn make_test_voices() -> Vec<TtsVoiceInfo> {
    vec![TtsVoiceInfo {
        voice_id: "test_voice".into(),
        name: "Test".into(),
        gender: None,
        language: "zh".into(),
        styles: vec![],
        preview_url: None,
        scenarios: vec!["test".into()],
        model_version: "2.0".into(),
    }]
}

fn test_pool(ws_url: &str) -> VolcengineTtsPool {
    VolcengineTtsPool::new(
        Box::new(StaticCredential::new(
            "test-app",
            "test-token",
            "volc.service_type.10029",
        )),
        VoiceRegistry::new().with_voices(make_test_voices()),
        Box::new(NoOpPreprocessor),
        "test_voice",
        ws_url,
        24_000,
    )
}

fn session_config() -> TtsSessionConfig {
    TtsSessionConfig {
        voice_id: "test_voice".into(),
        sample_rate: 24_000,
        output_format: TtsOutputFormat::Pcm,
        format: AudioFormat {
            sample_rate: 24_000,
            channels: 1,
            output_format: TtsOutputFormat::Pcm,
        },
        ..TtsSessionConfig::default()
    }
}

fn full_server_frame(event: i32, id: &str, payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![
        PROTO_HEADER,
        (MSG_FULL_SERVER_RESP << 4) | FLAG_WITH_EVENT,
        SERIAL_JSON << 4,
        0x00,
    ];
    frame.extend_from_slice(&event.to_be_bytes());
    frame.extend_from_slice(&(id.len() as u32).to_be_bytes());
    frame.extend_from_slice(id.as_bytes());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn audio_server_frame(session_id: &str, audio: &[u8]) -> Vec<u8> {
    let mut frame = vec![
        PROTO_HEADER,
        (MSG_AUDIO_ONLY << 4) | FLAG_WITH_EVENT,
        SERIAL_RAW << 4,
        0x00,
    ];
    frame.extend_from_slice(&EVT_TTS_RESPONSE.to_be_bytes());
    frame.extend_from_slice(&(session_id.len() as u32).to_be_bytes());
    frame.extend_from_slice(session_id.as_bytes());
    frame.extend_from_slice(&(audio.len() as u32).to_be_bytes());
    frame.extend_from_slice(audio);
    frame
}

async fn send_audio_session(write: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, WsMessage>, session_label: &str) {
    let started = full_server_frame(EVT_SESSION_STARTED, session_label, b"{}");
    let _ = write.send(WsMessage::Binary(started.into())).await;

    let audio = [0_i16.to_le_bytes(), 32767_i16.to_le_bytes()].concat();
    let audio_frame = audio_server_frame(session_label, &audio);
    let _ = write.send(WsMessage::Binary(audio_frame.into())).await;

    let finished = full_server_frame(EVT_SESSION_FINISHED, session_label, b"{}");
    let _ = write.send(WsMessage::Binary(finished.into())).await;
}

async fn spawn_basic_server() -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break; };
                    tokio::spawn(async move {
                        let Ok(ws) = accept_async(stream).await else { return; };
                        let (mut write, mut read) = ws.split();
                        let mut session_index = 0_usize;

                        while let Some(msg) = read.next().await {
                            let Ok(WsMessage::Binary(data)) = msg else { continue; };
                            if data.len() < 8 {
                                continue;
                            }

                            match i32::from_be_bytes([data[4], data[5], data[6], data[7]]) {
                                EVT_START_CONNECTION => {
                                    let frame = full_server_frame(EVT_CONNECTION_STARTED, "conn-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                }
                                EVT_START_SESSION => {
                                    session_index += 1;
                                    let session_label = format!("session-{session_index}");
                                    send_audio_session(&mut write, &session_label).await;
                                }
                                EVT_FINISH_CONNECTION => break,
                                EVT_FINISH_SESSION => {}
                                _ => {}
                            }
                        }
                    });
                }
                _ = &mut shutdown_rx => break,
            }
        }
    });

    (format!("ws://{}", addr), shutdown_tx)
}

async fn spawn_queue_server(release_rx: oneshot::Receiver<()>) -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut release_rx = Some(release_rx);
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break; };
                    let release_rx = release_rx.take();
                    tokio::spawn(async move {
                        let Ok(ws) = accept_async(stream).await else { return; };
                        let (mut write, mut read) = ws.split();
                        let mut session_index = 0_usize;
                        let mut release_rx = release_rx;

                        while let Some(msg) = read.next().await {
                            let Ok(WsMessage::Binary(data)) = msg else { continue; };
                            if data.len() < 8 {
                                continue;
                            }

                            match i32::from_be_bytes([data[4], data[5], data[6], data[7]]) {
                                EVT_START_CONNECTION => {
                                    let frame = full_server_frame(EVT_CONNECTION_STARTED, "conn-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                }
                                EVT_START_SESSION => {
                                    session_index += 1;
                                    let session_label = format!("session-{session_index}");
                                    let started = full_server_frame(EVT_SESSION_STARTED, &session_label, b"{}");
                                    let _ = write.send(WsMessage::Binary(started.into())).await;

                                    let audio = [0_i16.to_le_bytes(), 32767_i16.to_le_bytes()].concat();
                                    let audio_frame = audio_server_frame(&session_label, &audio);
                                    let _ = write.send(WsMessage::Binary(audio_frame.into())).await;

                                    if session_index == 1 {
                                        if let Some(rx) = release_rx.take() {
                                            let _ = rx.await;
                                        }
                                    }

                                    let finished = full_server_frame(EVT_SESSION_FINISHED, &session_label, b"{}");
                                    let _ = write.send(WsMessage::Binary(finished.into())).await;
                                }
                                EVT_FINISH_CONNECTION => break,
                                EVT_FINISH_SESSION => {}
                                _ => {}
                            }
                        }
                    });
                }
                _ = &mut shutdown_rx => break,
            }
        }
    });

    (format!("ws://{}", addr), shutdown_tx)
}

async fn spawn_stalled_session_server() -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break; };
                    tokio::spawn(async move {
                        let Ok(ws) = accept_async(stream).await else { return; };
                        let (mut write, mut read) = ws.split();
                        let mut started_first = false;

                        while let Some(msg) = read.next().await {
                            let Ok(WsMessage::Binary(data)) = msg else { continue; };
                            if data.len() < 8 {
                                continue;
                            }

                            match i32::from_be_bytes([data[4], data[5], data[6], data[7]]) {
                                EVT_START_CONNECTION => {
                                    let frame = full_server_frame(EVT_CONNECTION_STARTED, "conn-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                }
                                EVT_START_SESSION if !started_first => {
                                    started_first = true;
                                    let frame = full_server_frame(EVT_SESSION_STARTED, "session-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                }
                                EVT_FINISH_CONNECTION => break,
                                _ => {}
                            }
                        }
                    });
                }
                _ = &mut shutdown_rx => break,
            }
        }
    });

    (format!("ws://{}", addr), shutdown_tx)
}

async fn spawn_reconnecting_server(connection_count: Arc<AtomicUsize>) -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break; };
                    let connection_count = Arc::clone(&connection_count);
                    tokio::spawn(async move {
                        connection_count.fetch_add(1, Ordering::SeqCst);
                        let Ok(ws) = accept_async(stream).await else { return; };
                        let (mut write, mut read) = ws.split();

                        while let Some(msg) = read.next().await {
                            let Ok(WsMessage::Binary(data)) = msg else { continue; };
                            if data.len() < 8 {
                                continue;
                            }

                            match i32::from_be_bytes([data[4], data[5], data[6], data[7]]) {
                                EVT_START_CONNECTION => {
                                    let frame = full_server_frame(EVT_CONNECTION_STARTED, "conn-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                }
                                EVT_START_SESSION => {
                                    send_audio_session(&mut write, "session-1").await;
                                    let _ = write.send(WsMessage::Close(None)).await;
                                    break;
                                }
                                EVT_FINISH_CONNECTION => break,
                                _ => {}
                            }
                        }
                    });
                }
                _ = &mut shutdown_rx => break,
            }
        }
    });

    (format!("ws://{}", addr), shutdown_tx)
}

async fn recv_audio_frame(
    rx: &mut mpsc::Receiver<Result<AudioFrame, xz_tts::XzTtsError>>,
    timeout_secs: u64,
) -> AudioFrame {
    timeout(Duration::from_secs(timeout_secs), rx.recv())
        .await
        .expect("audio frame should arrive before timeout")
        .expect("audio channel should stay open")
        .expect("audio frame should be ok")
}

#[tokio::test]
async fn pool_submit_queues_session() {
    let (release_tx, release_rx) = oneshot::channel();
    let (ws_url, shutdown) = spawn_queue_server(release_rx).await;
    let pool = test_pool(&ws_url);

    let (text_tx1, text_rx1) = mpsc::channel::<String>(1);
    let (text_tx2, text_rx2) = mpsc::channel::<String>(1);
    let mut audio_rx1 = pool.submit(text_rx1, session_config()).expect("first submit should work");
    let mut audio_rx2 = pool.submit(text_rx2, session_config()).expect("second submit should work");

    drop(text_tx1);
    drop(text_tx2);

    let first = recv_audio_frame(&mut audio_rx1, 2).await;
    assert_eq!(first.format.sample_rate, 24_000);
    assert!(timeout(Duration::from_millis(250), audio_rx2.recv()).await.is_err());

    let _ = release_tx.send(());

    let second = recv_audio_frame(&mut audio_rx2, 2).await;
    assert_eq!(second.samples.len(), 2);

    pool.shutdown();
    let _ = shutdown.send(());
}

#[tokio::test]
async fn pool_implements_streaming_trait() {
    let (ws_url, shutdown) = spawn_basic_server().await;
    let pool = test_pool(&ws_url);
    let tts: &dyn StreamingTts = &pool;

    let (_text_tx, text_rx) = mpsc::channel::<String>(1);
    let mut audio_rx = tts
        .synthesize_streaming_with_config(text_rx, session_config())
        .await
        .expect("trait submit should work");

    let frame = recv_audio_frame(&mut audio_rx, 2).await;
    assert_eq!(frame.format.channels, 1);
    assert_eq!(tts.available_voices().len(), 1);
    assert_eq!(tts.available_voices()[0].voice_id, "test_voice");

    pool.shutdown();
    let _ = shutdown.send(());
}

#[tokio::test]
async fn pool_shutdown_drops_pending() {
    let (ws_url, shutdown) = spawn_stalled_session_server().await;
    let pool = test_pool(&ws_url);

    let (text_tx1, text_rx1) = mpsc::channel::<String>(1);
    let (text_tx2, text_rx2) = mpsc::channel::<String>(1);
    let mut audio_rx1 = pool.submit(text_rx1, session_config()).expect("first submit should work");
    let mut audio_rx2 = pool.submit(text_rx2, session_config()).expect("second submit should work");

    drop(text_tx1);
    drop(text_tx2);

    tokio::time::sleep(Duration::from_millis(200)).await;
    pool.shutdown();

    assert!(timeout(Duration::from_secs(2), audio_rx1.recv()).await.unwrap().is_none());
    assert!(timeout(Duration::from_secs(2), audio_rx2.recv()).await.unwrap().is_none());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn pool_reconnects_after_disconnect() {
    let connection_count = Arc::new(AtomicUsize::new(0));
    let (ws_url, shutdown) = spawn_reconnecting_server(Arc::clone(&connection_count)).await;
    let pool = test_pool(&ws_url);

    let (text_tx1, text_rx1) = mpsc::channel::<String>(1);
    let mut audio_rx1 = pool.submit(text_rx1, session_config()).expect("first submit should work");
    drop(text_tx1);
    let _ = recv_audio_frame(&mut audio_rx1, 2).await;

    let (text_tx2, text_rx2) = mpsc::channel::<String>(1);
    let mut audio_rx2 = pool.submit(text_rx2, session_config()).expect("second submit should work");
    drop(text_tx2);
    let _ = recv_audio_frame(&mut audio_rx2, 5).await;

    assert!(connection_count.load(Ordering::SeqCst) >= 2);

    pool.shutdown();
    let _ = shutdown.send(());
}

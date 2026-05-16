#[path = "common/mod.rs"]
mod common;

use common::spawn_mock_server;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};
use xz_tts::client::VolcengineTtsClient;
use xz_tts::credential::StaticCredential;
use xz_tts::preprocess::NoOpPreprocessor;
use xz_tts::protocol::{
    build_connection_frame, EVT_CONNECTION_STARTED, EVT_FINISH_CONNECTION, EVT_FINISH_SESSION,
    EVT_SESSION_FINISHED, EVT_SESSION_STARTED, EVT_START_CONNECTION, EVT_START_SESSION,
    EVT_TTS_RESPONSE, FLAG_WITH_EVENT, MSG_AUDIO_ONLY, MSG_FULL_SERVER_RESP, PROTO_HEADER,
    SERIAL_JSON, SERIAL_RAW,
};
use xz_tts::types::{AudioFormat, AudioFrame, TtsSessionConfig};
use xz_tts::voices::VoiceRegistry;

fn test_client(ws_url: &str) -> VolcengineTtsClient {
    VolcengineTtsClient::new(
        Box::new(StaticCredential::new(
            "test-app",
            "test-token",
            "volc.service_type.10029",
        )),
        VoiceRegistry::new(),
        Box::new(NoOpPreprocessor),
        "test_voice",
        ws_url,
        24_000,
    )
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

async fn spawn_tts_mock_server() -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    let Ok((stream, _)) = result else { break; };
                    tokio::spawn(async move {
                        let Ok(ws) = accept_async(stream).await else { return; };
                        let (mut write, mut read) = ws.split();

                        while let Some(msg) = read.next().await {
                            let Ok(msg) = msg else { break; };
                            if let WsMessage::Binary(data) = msg {
                                if data.len() >= 8 && data[4..8] == EVT_START_CONNECTION.to_be_bytes() {
                                    let frame = full_server_frame(EVT_CONNECTION_STARTED, "conn-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                } else if data.len() >= 8 && data[4..8] == EVT_START_SESSION.to_be_bytes() {
                                    let frame = full_server_frame(EVT_SESSION_STARTED, "session-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                    let audio = [0_i16.to_le_bytes(), 32767_i16.to_le_bytes()].concat();
                                    let frame = audio_server_frame("session-1", &audio);
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                    let frame = full_server_frame(EVT_SESSION_FINISHED, "session-1", b"{}");
                                    let _ = write.send(WsMessage::Binary(frame.into())).await;
                                } else if data.len() >= 8 && data[4..8] == EVT_FINISH_CONNECTION.to_be_bytes() {
                                    break;
                                } else if data.len() >= 8 && data[4..8] == EVT_FINISH_SESSION.to_be_bytes() {
                                    continue;
                                }
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

#[tokio::test]
async fn client_creates_successfully() {
    let (addr, shutdown): (std::net::SocketAddr, oneshot::Sender<()>) = spawn_mock_server().await;
    let client = test_client(&format!("ws://{}", addr));

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    client.shutdown();
    let _ = shutdown.send(());
}

#[tokio::test]
async fn client_connects_to_mock_server() {
    let (addr, shutdown): (std::net::SocketAddr, oneshot::Sender<()>) = spawn_mock_server().await;
    let client = test_client(&format!("ws://{}", addr));

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    client.shutdown();
    let _ = shutdown.send(());
}

#[tokio::test]
async fn submit_session_produces_audio_frame() {
    let (ws_url, shutdown) = spawn_tts_mock_server().await;
    let client = test_client(&ws_url);

    let (text_tx, text_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<AudioFrame>(1);

    let result = client.submit_session(
        text_rx,
        audio_tx,
        TtsSessionConfig {
            voice_id: "test_voice".into(),
            sample_rate: 24_000,
            format: AudioFormat {
                sample_rate: 24_000,
                channels: 1,
            },
            ..Default::default()
        },
    );
    assert!(result.is_ok());

    drop(text_tx);

    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), audio_rx.recv())
        .await
        .expect("audio frame should arrive")
        .expect("audio channel should stay open");

    assert_eq!(frame.format.sample_rate, 24_000);
    assert_eq!(frame.format.channels, 1);
    assert_eq!(frame.samples.len(), 2);

    client.shutdown();
    let _ = shutdown.send(());
}

#[tokio::test]
async fn build_connection_frame_keeps_expected_shape() {
    let frame = build_connection_frame(EVT_START_CONNECTION);

    assert_eq!(&frame[..4], &[0x11, 0x14, 0x10, 0x00]);
    assert_eq!(&frame[4..8], &EVT_START_CONNECTION.to_be_bytes());
    assert_eq!(frame.len(), 14);
}

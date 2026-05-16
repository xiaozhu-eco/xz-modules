use xz_tts::error::XzTtsError;
use xz_tts::protocol::*;

#[test]
fn build_connection_frame_starts_with_header_and_has_expected_length() {
    let frame = build_connection_frame(1);

    assert_eq!(&frame[..4], &[0x11, 0x14, 0x10, 0x00]);
    assert_eq!(frame.len(), 14);
}

#[test]
fn build_session_frame_contains_session_id_and_header() {
    let frame = build_session_frame(100, "test", b"{}");

    assert_eq!(&frame[..4], &[0x11, 0x14, 0x10, 0x00]);
    assert!(frame.windows(4).any(|window| window == b"test"));
}

#[test]
fn parse_connection_started_fixture() {
    let frame = server_frame(EVT_CONNECTION_STARTED, Some("conn-1"), b"{}", SERIAL_JSON);

    match parse_server_frame(&frame) {
        Ok(ServerEvent::ConnectionStarted) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn parse_tts_response_fixture() {
    let frame = audio_only_frame(Some(EVT_TTS_RESPONSE), Some("sid-1"), &[0, 1, 2, 3]);

    match parse_server_frame(&frame) {
        Ok(ServerEvent::TtsAudioData { audio }) => assert_eq!(audio, vec![0, 1, 2, 3]),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn parse_truncated_frame_returns_error() {
    let err = parse_server_frame(&[0x11, 0x94, 0x10]).unwrap_err();

    match err {
        XzTtsError::Protocol { .. } => {}
        other => panic!("expected protocol error, got {other:?}"),
    }
}

#[test]
fn parse_error_frame_returns_protocol_error_event() {
    let frame = error_frame(401, br#"{"message":"invalid token"}"#);

    match parse_server_frame(&frame) {
        Ok(ServerEvent::ProtocolError { code, message }) => {
            assert_eq!(code, 401);
            assert_eq!(message, "invalid token");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

fn server_frame(event: i32, id: Option<&str>, payload: &[u8], serialization: u8) -> Vec<u8> {
    let id = id.unwrap_or("").as_bytes();
    let mut frame = vec![
        PROTO_HEADER,
        (MSG_FULL_SERVER_RESP << 4) | FLAG_WITH_EVENT,
        serialization << 4,
        0x00,
    ];
    frame.extend_from_slice(&event.to_be_bytes());
    if should_include_identifier(event) {
        frame.extend_from_slice(&(id.len() as u32).to_be_bytes());
        frame.extend_from_slice(id);
    }
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn audio_only_frame(event: Option<i32>, session_id: Option<&str>, audio: &[u8]) -> Vec<u8> {
    let mut frame = vec![
        PROTO_HEADER,
        (MSG_AUDIO_ONLY << 4) | if event.is_some() { FLAG_WITH_EVENT } else { 0 },
        SERIAL_RAW << 4,
        0x00,
    ];

    if let Some(event) = event {
        frame.extend_from_slice(&event.to_be_bytes());
        let session_id = session_id.unwrap_or("").as_bytes();
        frame.extend_from_slice(&(session_id.len() as u32).to_be_bytes());
        frame.extend_from_slice(session_id);
    }

    frame.extend_from_slice(&(audio.len() as u32).to_be_bytes());
    frame.extend_from_slice(audio);
    frame
}

fn error_frame(code: i32, payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![PROTO_HEADER, MSG_ERROR << 4, SERIAL_JSON << 4, 0x00];
    frame.extend_from_slice(&code.to_be_bytes());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn should_include_identifier(event: i32) -> bool {
    matches!(
        event,
        EVT_CONNECTION_STARTED
            | EVT_CONNECTION_FAILED
            | EVT_CONNECTION_FINISHED
            | EVT_SESSION_STARTED
            | EVT_SESSION_CANCELED
            | EVT_SESSION_FINISHED
            | EVT_SESSION_FAILED
            | EVT_TTS_SENTENCE_START
            | EVT_TTS_SENTENCE_END
            | EVT_TTS_RESPONSE
    )
}

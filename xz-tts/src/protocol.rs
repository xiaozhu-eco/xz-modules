use crate::error::XzTtsError;

pub const PROTO_HEADER: u8 = 0x11;

pub const MSG_FULL_CLIENT_REQ: u8 = 0x1;
pub const MSG_FULL_SERVER_RESP: u8 = 0x9;
pub const MSG_AUDIO_ONLY: u8 = 0xB;
pub const MSG_ERROR: u8 = 0xF;

pub const FLAG_WITH_EVENT: u8 = 0x4;

pub const SERIAL_RAW: u8 = 0x0;
pub const SERIAL_JSON: u8 = 0x1;

pub const EVT_START_CONNECTION: i32 = 1;
pub const EVT_FINISH_CONNECTION: i32 = 2;
pub const EVT_START_SESSION: i32 = 100;
pub const EVT_CANCEL_SESSION: i32 = 101;
pub const EVT_FINISH_SESSION: i32 = 102;
pub const EVT_TASK_REQUEST: i32 = 200;

pub const EVT_CONNECTION_STARTED: i32 = 50;
pub const EVT_CONNECTION_FAILED: i32 = 51;
pub const EVT_CONNECTION_FINISHED: i32 = 52;
pub const EVT_SESSION_STARTED: i32 = 150;
pub const EVT_SESSION_CANCELED: i32 = 151;
pub const EVT_SESSION_FINISHED: i32 = 152;
pub const EVT_SESSION_FAILED: i32 = 153;
pub const EVT_TTS_SENTENCE_START: i32 = 350;
pub const EVT_TTS_SENTENCE_END: i32 = 351;
pub const EVT_TTS_RESPONSE: i32 = 352;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEvent {
    ConnectionStarted,
    ConnectionFailed { message: String },
    ConnectionFinished,
    SessionStarted,
    SessionFinished,
    SessionFailed { message: String },
    SessionCanceled,
    TtsSentenceStart,
    TtsSentenceEnd,
    TtsAudioData { audio: Vec<u8> },
    ProtocolError { code: i32, message: String },
    Unknown { event: i32 },
}

pub fn build_connection_frame(event: i32) -> Vec<u8> {
    let payload = b"{}";
    let mut buf = Vec::with_capacity(4 + 4 + 4 + payload.len());
    buf.push(PROTO_HEADER);
    buf.push((MSG_FULL_CLIENT_REQ << 4) | FLAG_WITH_EVENT);
    buf.push(SERIAL_JSON << 4);
    buf.push(0x00);
    buf.extend_from_slice(&event.to_be_bytes());
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}

pub fn build_session_frame(event: i32, session_id: &str, payload: &[u8]) -> Vec<u8> {
    let session_id = session_id.as_bytes();
    let mut buf = Vec::with_capacity(4 + 4 + 4 + session_id.len() + 4 + payload.len());
    buf.push(PROTO_HEADER);
    buf.push((MSG_FULL_CLIENT_REQ << 4) | FLAG_WITH_EVENT);
    buf.push(SERIAL_JSON << 4);
    buf.push(0x00);
    buf.extend_from_slice(&event.to_be_bytes());
    buf.extend_from_slice(&(session_id.len() as u32).to_be_bytes());
    buf.extend_from_slice(session_id);
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}

pub fn parse_server_frame(data: &[u8]) -> Result<ServerEvent, XzTtsError> {
    if data.len() < 4 {
        return Err(protocol_error(-1, format!("frame too short: {} bytes", data.len())));
    }

    if data[0] != PROTO_HEADER {
        return Err(protocol_error(
            -1,
            format!("invalid protocol header: expected 0x{PROTO_HEADER:02X}, got 0x{:02X}", data[0]),
        ));
    }

    let msg_type = (data[1] >> 4) & 0x0F;
    let flags = data[1] & 0x0F;
    let serialization = (data[2] >> 4) & 0x0F;

    let mut pos = 4;

    if msg_type == MSG_ERROR {
        let code = read_i32_be(data, &mut pos)?;
        let payload = read_remaining_payload(data, &mut pos);
        let message = decode_message_payload(&payload, serialization)
            .unwrap_or_else(|| format!("protocol error {code}"));
        return Ok(ServerEvent::ProtocolError { code, message });
    }

    if msg_type == MSG_AUDIO_ONLY {
        let event = if flags & FLAG_WITH_EVENT != 0 {
            read_i32_be(data, &mut pos)?
        } else {
            EVT_TTS_RESPONSE
        };

        if flags & FLAG_WITH_EVENT != 0 {
            let _session_id = read_length_prefixed_string(data, &mut pos)?;
        }

        let audio = read_remaining_payload(data, &mut pos);
        return match event {
            EVT_TTS_RESPONSE => Ok(ServerEvent::TtsAudioData { audio }),
            _ => Ok(ServerEvent::Unknown { event }),
        };
    }

    if msg_type != MSG_FULL_SERVER_RESP {
        return Err(protocol_error(-1, format!("unsupported server message type: {msg_type}")));
    }

    let event = if flags & FLAG_WITH_EVENT != 0 {
        read_i32_be(data, &mut pos)?
    } else {
        -1
    };

    match event {
        EVT_CONNECTION_STARTED | EVT_CONNECTION_FAILED | EVT_CONNECTION_FINISHED => {
            let _connection_id = read_length_prefixed_string(data, &mut pos)?;
        }
        EVT_SESSION_STARTED
        | EVT_SESSION_CANCELED
        | EVT_SESSION_FINISHED
        | EVT_SESSION_FAILED
        | EVT_TTS_SENTENCE_START
        | EVT_TTS_SENTENCE_END
        | EVT_TTS_RESPONSE => {
            let _session_id = read_length_prefixed_string(data, &mut pos)?;
        }
        _ => {}
    }

    let payload = read_remaining_payload(data, &mut pos);

    match event {
        EVT_CONNECTION_STARTED => Ok(ServerEvent::ConnectionStarted),
        EVT_CONNECTION_FAILED => Ok(ServerEvent::ConnectionFailed {
            message: decode_message_payload(&payload, serialization).unwrap_or_default(),
        }),
        EVT_CONNECTION_FINISHED => Ok(ServerEvent::ConnectionFinished),
        EVT_SESSION_STARTED => Ok(ServerEvent::SessionStarted),
        EVT_SESSION_CANCELED => Ok(ServerEvent::SessionCanceled),
        EVT_SESSION_FINISHED => Ok(ServerEvent::SessionFinished),
        EVT_SESSION_FAILED => Ok(ServerEvent::SessionFailed {
            message: decode_message_payload(&payload, serialization).unwrap_or_default(),
        }),
        EVT_TTS_SENTENCE_START => Ok(ServerEvent::TtsSentenceStart),
        EVT_TTS_SENTENCE_END => Ok(ServerEvent::TtsSentenceEnd),
        EVT_TTS_RESPONSE => Ok(ServerEvent::TtsAudioData { audio: payload }),
        _ => Ok(ServerEvent::Unknown { event }),
    }
}

fn read_i32_be(data: &[u8], pos: &mut usize) -> Result<i32, XzTtsError> {
    if *pos + 4 > data.len() {
        return Err(protocol_error(
            -1,
            format!("unable to read i32 at offset {} from {} bytes", *pos, data.len()),
        ));
    }

    let value = i32::from_be_bytes(data[*pos..*pos + 4].try_into().expect("slice length checked"));
    *pos += 4;
    Ok(value)
}

fn read_length_prefixed_string(data: &[u8], pos: &mut usize) -> Result<String, XzTtsError> {
    if *pos + 4 > data.len() {
        return Err(protocol_error(
            -1,
            format!("unable to read string length at offset {} from {} bytes", *pos, data.len()),
        ));
    }

    let len = u32::from_be_bytes(data[*pos..*pos + 4].try_into().expect("slice length checked")) as usize;
    *pos += 4;

    if *pos + len > data.len() {
        return Err(protocol_error(
            -1,
            format!(
                "string length {} at offset {} exceeds frame size {}",
                len, *pos, data.len()
            ),
        ));
    }

    let value = String::from_utf8_lossy(&data[*pos..*pos + len]).to_string();
    *pos += len;
    Ok(value)
}

fn read_remaining_payload(data: &[u8], pos: &mut usize) -> Vec<u8> {
    if *pos + 4 > data.len() {
        return Vec::new();
    }

    let len = u32::from_be_bytes(data[*pos..*pos + 4].try_into().expect("slice length checked")) as usize;
    *pos += 4;

    let available = data.len().saturating_sub(*pos);
    let take = len.min(available);
    let payload = data[*pos..*pos + take].to_vec();
    *pos += take;
    payload
}

fn decode_message_payload(payload: &[u8], serialization: u8) -> Option<String> {
    if payload.is_empty() {
        return None;
    }

    if serialization == SERIAL_JSON {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) {
            if let Some(message) = value.get("message").and_then(|value| value.as_str()) {
                return Some(message.to_string());
            }
            if let Some(message) = value.as_str() {
                return Some(message.to_string());
            }
            return Some(value.to_string());
        }
    }

    Some(String::from_utf8_lossy(payload).to_string())
}

fn protocol_error(code: i32, message: impl Into<String>) -> XzTtsError {
    XzTtsError::Protocol {
        code,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_connection_frame() {
        let frame = build_connection_frame(1);

        assert_eq!(&frame[..4], &[0x11, 0x14, 0x10, 0x00]);
        assert_eq!(frame.len(), 14);
        assert_eq!(&frame[4..8], &1_i32.to_be_bytes());
    }

    #[test]
    fn test_build_session_frame() {
        let frame = build_session_frame(100, "test", b"{}");

        assert_eq!(&frame[..4], &[0x11, 0x14, 0x10, 0x00]);
        assert!(frame.windows(4).any(|window| window == b"test"));
        assert_eq!(&frame[4..8], &100_i32.to_be_bytes());
    }

    #[test]
    fn parse_connection_started_fixture() {
        let frame = full_server_frame(EVT_CONNECTION_STARTED, Some("conn-1"), b"{}", SERIAL_JSON);
        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::ConnectionStarted);
    }

    #[test]
    fn parse_connection_failed_fixture() {
        let frame = full_server_frame(
            EVT_CONNECTION_FAILED,
            Some("conn-1"),
            br#"{"message":"unauthorized"}"#,
            SERIAL_JSON,
        );

        assert_eq!(
            parse_server_frame(&frame).unwrap(),
            ServerEvent::ConnectionFailed {
                message: "unauthorized".into(),
            }
        );
    }

    #[test]
    fn parse_connection_finished_fixture() {
        let frame = full_server_frame(EVT_CONNECTION_FINISHED, Some("conn-1"), b"{}", SERIAL_JSON);
        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::ConnectionFinished);
    }

    #[test]
    fn parse_session_started_fixture() {
        let frame = full_server_frame(EVT_SESSION_STARTED, Some("sid-1"), b"{}", SERIAL_JSON);
        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::SessionStarted);
    }

    #[test]
    fn parse_session_canceled_fixture() {
        let frame = full_server_frame(EVT_SESSION_CANCELED, Some("sid-1"), b"{}", SERIAL_JSON);
        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::SessionCanceled);
    }

    #[test]
    fn parse_session_finished_fixture() {
        let frame = full_server_frame(
            EVT_SESSION_FINISHED,
            Some("sid-1"),
            br#"{"status_code":20000000,"message":"ok"}"#,
            SERIAL_JSON,
        );

        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::SessionFinished);
    }

    #[test]
    fn parse_session_failed_fixture() {
        let frame = full_server_frame(
            EVT_SESSION_FAILED,
            Some("sid-1"),
            br#"{"message":"quota exceeded"}"#,
            SERIAL_JSON,
        );

        assert_eq!(
            parse_server_frame(&frame).unwrap(),
            ServerEvent::SessionFailed {
                message: "quota exceeded".into(),
            }
        );
    }

    #[test]
    fn parse_sentence_start_fixture() {
        let frame = full_server_frame(EVT_TTS_SENTENCE_START, Some("sid-1"), b"{}", SERIAL_JSON);
        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::TtsSentenceStart);
    }

    #[test]
    fn parse_sentence_end_fixture() {
        let frame = full_server_frame(EVT_TTS_SENTENCE_END, Some("sid-1"), b"{}", SERIAL_JSON);
        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::TtsSentenceEnd);
    }

    #[test]
    fn parse_tts_response_fixture() {
        let frame = audio_only_frame(Some(EVT_TTS_RESPONSE), Some("sid-1"), &[0, 1, 2, 3]);

        assert_eq!(
            parse_server_frame(&frame).unwrap(),
            ServerEvent::TtsAudioData {
                audio: vec![0, 1, 2, 3],
            }
        );
    }

    #[test]
    fn parse_error_frame_fixture() {
        let frame = error_frame(401, br#"{"message":"invalid token"}"#);

        assert_eq!(
            parse_server_frame(&frame).unwrap(),
            ServerEvent::ProtocolError {
                code: 401,
                message: "invalid token".into(),
            }
        );
    }

    #[test]
    fn parse_unknown_event_fixture() {
        let frame = full_server_frame(999, None, b"{}", SERIAL_JSON);
        assert_eq!(parse_server_frame(&frame).unwrap(), ServerEvent::Unknown { event: 999 });
    }

    #[test]
    fn truncated_frame_returns_protocol_error() {
        let err = parse_server_frame(&[0x11, 0x94, 0x10]).unwrap_err();
        match err {
            XzTtsError::Protocol { .. } => {}
            other => panic!("expected protocol error, got {other:?}"),
        }
    }

    #[test]
    fn wrong_payload_length_is_handled_gracefully() {
        let mut frame = audio_only_frame(Some(EVT_TTS_RESPONSE), Some("sid-1"), &[0, 1, 2, 3]);
        let len_index = frame.len() - 8;
        frame[len_index..len_index + 4].copy_from_slice(&8_u32.to_be_bytes());

        assert_eq!(
            parse_server_frame(&frame).unwrap(),
            ServerEvent::TtsAudioData {
                audio: vec![0, 1, 2, 3],
            }
        );
    }

    fn full_server_frame(event: i32, id: Option<&str>, payload: &[u8], serialization: u8) -> Vec<u8> {
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
}

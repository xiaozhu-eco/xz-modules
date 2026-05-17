use crate::async_types::{
    AsyncQueryRequest, AsyncQueryResponse, AsyncSubmitReqParams, AsyncSubmitRequest,
    AsyncSubmitResponse, AsyncUser, AudioParams, CompletedTask, DownloadedAudio, TaskStatus,
};
use crate::credential::CredentialProvider;
use crate::error::XzTtsError;
use crate::http_client;
use crate::types::TtsSessionConfig;
use serde_json::{Value, json};

const MAX_SUBMIT_TEXT_CHARS: usize = 100_000;

pub struct AsyncTtsClient {
    credential_provider: Box<dyn CredentialProvider>,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct AsyncTtsSubmitConfig {
    pub text: String,
    pub ssml: Option<String>,
    pub uid: Option<String>,
    pub unique_id: Option<String>,
    pub model: Option<String>,
    pub tts_config: TtsSessionConfig,
}

impl AsyncTtsClient {
    pub fn new(credential_provider: Box<dyn CredentialProvider>, base_url: impl Into<String>) -> Self {
        Self {
            credential_provider,
            base_url: base_url.into(),
        }
    }

    pub async fn submit(
        &self,
        config: &AsyncTtsSubmitConfig,
    ) -> Result<AsyncSubmitResponse, XzTtsError> {
        let payload_text = build_submit_text(config);
        let text_len = payload_text.chars().count();
        if text_len > MAX_SUBMIT_TEXT_CHARS {
            return Err(XzTtsError::TextTooLong {
                len: text_len,
                max: MAX_SUBMIT_TEXT_CHARS,
            });
        }

        let credential = self.credential_provider.resolve().await?;
        let request = AsyncSubmitRequest {
            user: AsyncUser {
                uid: config.uid.clone().unwrap_or_else(|| "anonymous".into()),
            },
            namespace: "BidirectionalTTS".into(),
            unique_id: config.unique_id.clone(),
            req_params: AsyncSubmitReqParams {
                text: config.ssml.as_ref().map(|_| None).unwrap_or_else(|| Some(config.text.clone())),
                ssml: config.ssml.as_ref().map(|_| payload_text.clone()),
                speaker: config.tts_config.voice_id.clone(),
                audio_params: AudioParams {
                    format: config.tts_config.output_format.to_string(),
                    sample_rate: config.tts_config.sample_rate,
                    speech_rate: config.tts_config.speech_rate,
                    emotion: config.tts_config.emotion_tag.clone(),
                    loudness_rate: config.tts_config.loudness_rate,
                },
                additions: build_additions(&config.tts_config),
                model: config.model.clone(),
                mix_speaker: build_mix_speaker(&config.tts_config),
            },
        };

        let url = format!("{}/api/v3/tts/submit", self.base_url.trim_end_matches('/'));
        let response = http_client::post_json(&url, &credential, &request).await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            let message = format!("HTTP {}: {}", status.as_u16(), body);
            return if status.as_u16() == 401 {
                Err(XzTtsError::Auth { message })
            } else if status.is_client_error() {
                Err(XzTtsError::Protocol {
                    code: status.as_u16() as i32,
                    message,
                })
            } else {
                Err(XzTtsError::Network { message })
            };
        }

        response.json::<AsyncSubmitResponse>().await.map_err(|err| XzTtsError::Format {
            message: format!("failed to deserialize async submit response: {err}"),
        })
    }

    pub async fn query(&self, task_id: &str) -> Result<AsyncQueryResponse, XzTtsError> {
        let credential = self.credential_provider.resolve().await?;
        let url = format!("{}/api/v3/tts/query", self.base_url.trim_end_matches('/'));
        let response = http_client::post_json(&url, &credential, &AsyncQueryRequest {
            task_id: task_id.to_string(),
        })
        .await?;

        map_json_response(response, "async query").await
    }

    pub async fn poll_until_complete(
        &self,
        task_id: &str,
        max_polls: u32,
        poll_interval_ms: u64,
    ) -> Result<CompletedTask, XzTtsError> {
        let max_polls = if max_polls == 0 { 120 } else { max_polls };
        let poll_interval_ms = if poll_interval_ms == 0 { 1000 } else { poll_interval_ms };

        for attempt in 0..max_polls {
            let response = self.query(task_id).await?;
            match response.data.task_status {
                TaskStatus::Running => {
                    if attempt + 1 == max_polls {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(poll_interval_ms)).await;
                }
                TaskStatus::Success => {
                    return Ok(CompletedTask {
                        audio_url: response.data.audio_url.ok_or_else(|| XzTtsError::Format {
                            message: "missing audio_url in successful async query response".into(),
                        })?,
                        sentences: response.data.sentences.unwrap_or_default(),
                        url_expire_time: response.data.url_expire_time.ok_or_else(|| XzTtsError::Format {
                            message: "missing url_expire_time in successful async query response".into(),
                        })?,
                    });
                }
                TaskStatus::Failure => {
                    return Err(XzTtsError::Internal {
                        message: format!("async task {task_id} failed: {}", response.message),
                    });
                }
            }
        }

        Err(XzTtsError::Timeout {
            message: format!("async task {task_id} did not complete after {max_polls} polls"),
        })
    }

    pub async fn download_audio(&self, audio_url: &str) -> Result<Vec<u8>, XzTtsError> {
        http_client::download_bytes(audio_url).await
    }

    pub async fn submit_and_wait(
        &self,
        config: &AsyncTtsSubmitConfig,
    ) -> Result<DownloadedAudio, XzTtsError> {
        let submit_response = self.submit(config).await?;
        let completed = self
            .poll_until_complete(&submit_response.data.task_id, 120, 1000)
            .await?;
        let audio_bytes = self.download_audio(&completed.audio_url).await?;

        Ok(DownloadedAudio {
            audio_bytes,
            format: config.tts_config.output_format.to_string(),
            sentences: completed.sentences,
        })
    }
}

fn build_submit_text(config: &AsyncTtsSubmitConfig) -> String {
    match &config.ssml {
        Some(ssml) if ssml.trim_start().starts_with("<speak>") => ssml.clone(),
        Some(ssml) => format!("<speak>{ssml}</speak>"),
        None => config.text.clone(),
    }
}

fn map_json_response<T>(response: reqwest::Response, label: &str) -> impl std::future::Future<Output = Result<T, XzTtsError>>
where
    T: serde::de::DeserializeOwned,
{
    async move {
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            let message = format!("HTTP {}: {}", status.as_u16(), body);
            return if status.as_u16() == 401 {
                Err(XzTtsError::Auth { message })
            } else if status.is_client_error() {
                Err(XzTtsError::Protocol {
                    code: status.as_u16() as i32,
                    message,
                })
            } else {
                Err(XzTtsError::Network { message })
            };
        }

        response.json::<T>().await.map_err(|err| XzTtsError::Format {
            message: format!("failed to deserialize {label} response: {err}"),
        })
    }
}

fn build_additions(config: &TtsSessionConfig) -> Option<Value> {
    let mut additions = json!({
        "disable_markdown_filter": config.disable_markdown_filter,
    });

    if let Some(pitch) = config.pitch {
        additions["post_process"] = json!({ "pitch": pitch });
    }
    if let Some(value) = config.enable_language_detector {
        additions["enable_language_detector"] = json!(value);
    }
    if let Some(value) = config.explicit_language.as_ref().filter(|value| !value.is_empty()) {
        additions["explicit_language"] = json!(value);
    }
    if let Some(value) = config.context_language.as_ref().filter(|value| !value.is_empty()) {
        additions["context_language"] = json!(value);
    }
    if let Some(value) = config.unsupported_char_ratio_thresh {
        additions["unsupported_char_ratio_thresh"] = json!(value);
    }
    if let Some(value) = config.aigc_watermark {
        additions["aigc_watermark"] = json!(value);
    }
    if let Some(value) = config.enable_latex_tn {
        additions["enable_latex_tn"] = json!(value);
    }
    if let Some(value) = config.mute_cut_threshold.as_ref().filter(|value| !value.is_empty()) {
        additions["mute_cut_threshold"] = json!(value);
    }
    if let Some(value) = config.mute_cut_remain_ms.as_ref().filter(|value| !value.is_empty()) {
        additions["mute_cut_remain_ms"] = json!(value);
    }

    Some(additions)
}

#[cfg(feature = "voice-mix")]
fn build_mix_speaker(config: &TtsSessionConfig) -> Option<Value> {
    (!config.mix_speakers.is_empty()).then(|| {
        json!({
            "speakers": config
                .mix_speakers
                .iter()
                .map(|speaker| json!({
                    "source_speaker": speaker.source_speaker,
                    "mix_factor": speaker.mix_factor,
                }))
                .collect::<Vec<_>>()
        })
    })
}

#[cfg(not(feature = "voice-mix"))]
fn build_mix_speaker(_config: &TtsSessionConfig) -> Option<Value> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::async_types::{SubmitData, TaskStatus};
    use crate::credential::{CredentialProvider, ResolvedTtsCredential};
    use crate::types::{AudioFormat, TtsOutputFormat};
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    enum ResponseBody {
        Json(Value),
        Bytes(Vec<u8>),
    }

    struct ResponseSpec {
        status_line: String,
        content_type: String,
        body: ResponseBody,
    }

    impl ResponseSpec {
        fn json(status_line: &str, body: Value) -> Self {
            Self {
                status_line: status_line.to_string(),
                content_type: "application/json".into(),
                body: ResponseBody::Json(body),
            }
        }

        fn bytes(status_line: &str, body: Vec<u8>, content_type: &str) -> Self {
            Self {
                status_line: status_line.to_string(),
                content_type: content_type.to_string(),
                body: ResponseBody::Bytes(body),
            }
        }
    }

    #[derive(Clone)]
    struct TestCredentialProvider;

    #[async_trait]
    impl CredentialProvider for TestCredentialProvider {
        async fn resolve(&self) -> Result<ResolvedTtsCredential, XzTtsError> {
            Ok(ResolvedTtsCredential {
                app_id: "app-id".into(),
                access_token: "access-token".into(),
                resource_id: "resource-id".into(),
            })
        }
    }

    struct TestServer {
        base_url: String,
        request_body: Arc<Mutex<Option<Value>>>,
        request_path: Arc<Mutex<Option<String>>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn spawn(status_line: &str, response_body: Value) -> Self {
            Self::spawn_sequence_specs(vec![ResponseSpec::json(status_line, response_body)])
        }

        fn spawn_sequence(responses: Vec<(&str, Value)>) -> Self {
            Self::spawn_sequence_specs(
                responses
                    .into_iter()
                    .map(|(status_line, body)| ResponseSpec::json(status_line, body))
                    .collect(),
            )
        }

        fn spawn_bytes(status_line: &str, response_body: Vec<u8>, content_type: &str) -> Self {
            Self::spawn_sequence_specs(vec![ResponseSpec::bytes(
                status_line,
                response_body,
                content_type,
            )])
        }

        fn spawn_sequence_specs(responses: Vec<ResponseSpec>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
            let addr = listener.local_addr().expect("server addr");
            let request_body = Arc::new(Mutex::new(None));
            let request_path = Arc::new(Mutex::new(None));
            let request_body_clone = Arc::clone(&request_body);
            let request_path_clone = Arc::clone(&request_path);

            let handle = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = listener.accept().expect("accept request");
                    let mut buf = Vec::new();
                    let mut header_end = None;
                    loop {
                        let mut chunk = [0_u8; 4096];
                        let read = stream.read(&mut chunk).expect("read request");
                        if read == 0 {
                            break;
                        }
                        buf.extend_from_slice(&chunk[..read]);
                        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_end = Some(pos + 4);
                            break;
                        }
                    }

                    let header_end = header_end.expect("header end");
                    let headers = String::from_utf8_lossy(&buf[..header_end]);
                    let mut lines = headers.lines();
                    let request_line = lines.next().expect("request line");
                    let path = request_line
                        .split_whitespace()
                        .nth(1)
                        .expect("request path")
                        .to_string();
                    *request_path_clone.lock().expect("lock request path") = Some(path);

                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            if name.eq_ignore_ascii_case("content-length") {
                                Some(value.trim().parse::<usize>().expect("content length"))
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);

                    while buf.len() < header_end + content_length {
                        let mut chunk = vec![0_u8; header_end + content_length - buf.len()];
                        let read = stream.read(&mut chunk).expect("read body");
                        if read == 0 {
                            break;
                        }
                        buf.extend_from_slice(&chunk[..read]);
                    }

                    let body = &buf[header_end..header_end + content_length];
                    *request_body_clone.lock().expect("lock request body") = if body.is_empty() {
                        None
                    } else {
                        Some(serde_json::from_slice(body).expect("json body"))
                    };

                    let (body_bytes, body_len) = match response.body {
                        ResponseBody::Json(body) => {
                            let text = body.to_string().into_bytes();
                            let len = text.len();
                            (text, len)
                        }
                        ResponseBody::Bytes(body) => {
                            let len = body.len();
                            (body, len)
                        }
                    };
                    let headers = format!(
                        "HTTP/1.1 {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        response.status_line, response.content_type, body_len
                    );
                    stream.write_all(headers.as_bytes()).expect("write response headers");
                    stream.write_all(&body_bytes).expect("write response body");
                    stream.flush().expect("flush response");
                }
            });

            Self {
                base_url: format!("http://{}", addr),
                request_body,
                request_path,
                handle: Some(handle),
            }
        }

        fn request_body(&self) -> Value {
            self.request_body
                .lock()
                .expect("lock request body")
                .clone()
                .expect("captured request body")
        }

        fn request_path(&self) -> String {
            self.request_path
                .lock()
                .expect("lock request path")
                .clone()
                .expect("captured request path")
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                handle.join().expect("join test server");
            }
        }
    }

    fn submit_config(text: String) -> AsyncTtsSubmitConfig {
        AsyncTtsSubmitConfig {
            text,
            ssml: None,
            uid: Some("user-123".into()),
            unique_id: Some("unique-456".into()),
            model: Some("speech-model".into()),
            tts_config: TtsSessionConfig {
                voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
                emotion_tag: Some("happy".into()),
                speech_rate: Some(10),
                loudness_rate: Some(4),
                pitch: Some(6),
                context_text: Some("context".into()),
                voice_commands: vec!["command".into()],
                emotion_scale: Some(5),
                enable_timestamp: Some(true),
                silence_duration: Some(1200),
                enable_language_detector: Some(true),
                explicit_language: Some("zh-cn".into()),
                context_language: Some("en".into()),
                unsupported_char_ratio_thresh: Some(0.25),
                aigc_watermark: Some(false),
                enable_latex_tn: Some(true),
                mute_cut_threshold: Some("0.2".into()),
                mute_cut_remain_ms: Some("200".into()),
                #[cfg(feature = "voice-mix")]
                mix_speakers: vec![],
                sample_rate: 24_000,
                output_format: TtsOutputFormat::Mp3,
                format: AudioFormat {
                    sample_rate: 24_000,
                    channels: 1,
                    output_format: TtsOutputFormat::Mp3,
                },
                disable_markdown_filter: false,
            },
        }
    }

    fn success_response(req_text_length: u32) -> Value {
        json!({
            "code": 0,
            "message": "ok",
            "data": {
                "task_id": "task-123",
                "req_text_length": req_text_length,
                "task_status": 1
            }
        })
    }

    #[tokio::test]
    async fn submit_sends_correct_json() {
        let server = TestServer::spawn("200 OK", success_response(5));
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());

        let response = client.submit(&submit_config("hello".into())).await.expect("submit succeeds");

        assert_eq!(response, AsyncSubmitResponse {
            code: 0,
            message: "ok".into(),
            data: SubmitData {
                task_id: "task-123".into(),
                req_text_length: 5,
                task_status: TaskStatus::Running,
            },
        });

        assert_eq!(server.request_path(), "/api/v3/tts/submit");
        let body = server.request_body();
        assert_eq!(body["user"]["uid"], "user-123");
        assert_eq!(body["namespace"], "BidirectionalTTS");
        assert_eq!(body["unique_id"], "unique-456");
        assert_eq!(body["req_params"]["text"], "hello");
        assert_eq!(body["req_params"]["speaker"], "zh_female_xiaohe_uranus_bigtts");
        assert_eq!(body["req_params"]["model"], "speech-model");
        assert_eq!(body["req_params"]["audio_params"]["format"], "mp3");
        assert_eq!(body["req_params"]["audio_params"]["sample_rate"], 24_000);
        assert_eq!(body["req_params"]["audio_params"]["speech_rate"], 10);
        assert_eq!(body["req_params"]["audio_params"]["emotion"], "happy");
        assert_eq!(body["req_params"]["audio_params"]["loudness_rate"], 4);
        assert_eq!(body["req_params"]["additions"]["disable_markdown_filter"], false);
        assert_eq!(body["req_params"]["additions"]["post_process"]["pitch"], 6);
    }

    #[tokio::test]
    async fn submit_rejects_text_too_long() {
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), "http://127.0.0.1:1");
        let err = client
            .submit(&submit_config("a".repeat(MAX_SUBMIT_TEXT_CHARS + 1)))
            .await
            .expect_err("submit should reject text longer than 100k");

        assert_eq!(
            err.to_string(),
            format!(
                "text too long: {} chars (max {})",
                MAX_SUBMIT_TEXT_CHARS + 1,
                MAX_SUBMIT_TEXT_CHARS
            )
        );
        match err {
            XzTtsError::TextTooLong { len, max } => {
                assert_eq!(len, MAX_SUBMIT_TEXT_CHARS + 1);
                assert_eq!(max, MAX_SUBMIT_TEXT_CHARS);
            }
            other => panic!("expected TextTooLong, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn submit_accepts_exactly_100k() {
        let server = TestServer::spawn("200 OK", success_response(MAX_SUBMIT_TEXT_CHARS as u32));
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());

        let response = client
            .submit(&submit_config("a".repeat(MAX_SUBMIT_TEXT_CHARS)))
            .await
            .expect("100k chars should be accepted");

        assert_eq!(response.data.req_text_length, MAX_SUBMIT_TEXT_CHARS as u32);
        assert_eq!(server.request_body()["req_params"]["text"].as_str().unwrap().chars().count(), MAX_SUBMIT_TEXT_CHARS);
    }

    #[tokio::test]
    async fn submit_auth_failure_maps_correctly() {
        let server = TestServer::spawn("401 Unauthorized", json!({"error": "bad token"}));
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());

        let err = client
            .submit(&submit_config("hello".into()))
            .await
            .expect_err("401 should map to auth error");

        assert!(matches!(err, XzTtsError::Auth { .. }));
    }

    #[tokio::test]
    async fn ssml_text_sent_in_ssml_field() {
        let server = TestServer::spawn("200 OK", success_response(20));
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());
        let mut config = submit_config("ignored".into());
        config.ssml = Some("<speak>测试</speak>".into());

        client.submit(&config).await.expect("submit succeeds");

        let body = server.request_body();
        assert_eq!(body["req_params"]["ssml"], "<speak>测试</speak>");
        assert!(body["req_params"]["text"].is_null());
    }

    #[tokio::test]
    async fn plain_text_wrapped_in_speak() {
        let server = TestServer::spawn("200 OK", success_response(20));
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());
        let mut config = submit_config("ignored".into());
        config.ssml = Some("测试".into());

        client.submit(&config).await.expect("submit succeeds");

        let body = server.request_body();
        assert_eq!(body["req_params"]["ssml"], "<speak>测试</speak>");
    }

    #[tokio::test]
    async fn poll_returns_success_when_task_completes() {
        let server = TestServer::spawn_sequence(vec![
            (
                "200 OK",
                json!({"code":0,"message":"running","data":{"task_id":"task-123","task_status":1,"audio_url":null,"sentences":null,"req_text_length":2,"synthesize_text_length":0,"url_expire_time":null}}),
            ),
            (
                "200 OK",
                json!({"code":0,"message":"ok","data":{"task_id":"task-123","task_status":2,"audio_url":"https://example.com/audio.mp3","sentences":[{"text":"hi","startTime":0.0,"endTime":1.0,"words":[]}],"req_text_length":2,"synthesize_text_length":2,"url_expire_time":1735689600}}),
            ),
        ]);
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());

        let result = client
            .poll_until_complete("task-123", 5, 1)
            .await
            .expect("poll succeeds");

        assert_eq!(result.audio_url, "https://example.com/audio.mp3");
        assert_eq!(result.url_expire_time, 1_735_689_600);
        assert_eq!(result.sentences[0].text, "hi");
    }

    #[tokio::test]
    async fn poll_returns_timeout_after_max_polls() {
        let server = TestServer::spawn_sequence(vec![
            (
                "200 OK",
                json!({"code":0,"message":"running","data":{"task_id":"task-123","task_status":1,"audio_url":null,"sentences":null,"req_text_length":2,"synthesize_text_length":0,"url_expire_time":null}}),
            ),
            (
                "200 OK",
                json!({"code":0,"message":"running","data":{"task_id":"task-123","task_status":1,"audio_url":null,"sentences":null,"req_text_length":2,"synthesize_text_length":0,"url_expire_time":null}}),
            ),
            (
                "200 OK",
                json!({"code":0,"message":"running","data":{"task_id":"task-123","task_status":1,"audio_url":null,"sentences":null,"req_text_length":2,"synthesize_text_length":0,"url_expire_time":null}}),
            ),
        ]);
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());

        let err = client
            .poll_until_complete("task-123", 3, 1)
            .await
            .expect_err("poll should time out");

        assert!(matches!(err, XzTtsError::Timeout { .. }));
    }

    #[tokio::test]
    async fn poll_returns_error_when_task_fails() {
        let server = TestServer::spawn("200 OK", json!({"code":0,"message":"tts failed","data":{"task_id":"task-123","task_status":3,"audio_url":null,"sentences":null,"req_text_length":2,"synthesize_text_length":0,"url_expire_time":null}}));
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), server.base_url.clone());

        let err = client
            .poll_until_complete("task-123", 3, 1)
            .await
            .expect_err("poll should fail");

        assert!(matches!(err, XzTtsError::Internal { .. }));
    }

    #[tokio::test]
    async fn download_audio_returns_bytes() {
        let server = TestServer::spawn_bytes("200 OK", vec![0, 1, 2, 3], "audio/mpeg");
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), "http://127.0.0.1:1");

        let bytes = client
            .download_audio(&format!("{}/audio.mp3", server.base_url))
            .await
            .expect("download succeeds");

        assert_eq!(bytes, vec![0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn download_audio_handles_expired_url() {
        let server = TestServer::spawn_bytes("404 Not Found", b"expired".to_vec(), "text/plain");
        let client = AsyncTtsClient::new(Box::new(TestCredentialProvider), "http://127.0.0.1:1");

        let err = client
            .download_audio(&format!("{}/expired.mp3", server.base_url))
            .await
            .expect_err("download should fail");

        assert!(matches!(err, XzTtsError::Format { .. }));
    }
}

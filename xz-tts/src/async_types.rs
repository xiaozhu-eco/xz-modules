use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AsyncSubmitRequest {
    pub user: AsyncUser,
    pub namespace: String,
    pub unique_id: Option<String>,
    pub req_params: AsyncSubmitReqParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AsyncUser {
    pub uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AsyncSubmitReqParams {
    pub text: Option<String>,
    pub ssml: Option<String>,
    pub speaker: String,
    pub audio_params: AudioParams,
    pub additions: Option<serde_json::Value>,
    pub model: Option<String>,
    pub mix_speaker: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AudioParams {
    pub format: String,
    pub sample_rate: u32,
    pub speech_rate: Option<i32>,
    pub emotion: Option<String>,
    pub loudness_rate: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AsyncSubmitResponse {
    pub code: i32,
    pub message: String,
    pub data: SubmitData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SubmitData {
    pub task_id: String,
    pub req_text_length: u32,
    pub task_status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AsyncQueryRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub struct AsyncQueryResponse {
    pub code: i32,
    pub message: String,
    pub data: QueryData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub struct QueryData {
    pub task_id: String,
    pub task_status: TaskStatus,
    pub audio_url: Option<String>,
    pub sentences: Option<Vec<Sentence>>,
    pub req_text_length: u32,
    pub synthesize_text_length: u32,
    #[serde(default, deserialize_with = "deserialize_optional_i64_from_string_or_number")]
    pub url_expire_time: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompletedTask {
    pub audio_url: String,
    pub sentences: Vec<Sentence>,
    pub url_expire_time: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadedAudio {
    pub audio_bytes: Vec<u8>,
    pub format: String,
    pub sentences: Vec<Sentence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Sentence {
    pub text: String,
    #[serde(rename = "startTime")]
    pub start_time: f64,
    #[serde(rename = "endTime")]
    pub end_time: f64,
    pub words: Vec<Word>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Word {
    pub word: String,
    #[serde(rename = "startTime")]
    pub start_time: f64,
    #[serde(rename = "endTime")]
    pub end_time: f64,
    pub confidence: f64,
}

impl Sentence {
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    pub fn text_with_timestamps(&self) -> String {
        self.words
            .iter()
            .map(|word| format!("{}({:.3}-{:.3})", word.word, word.start_time, word.end_time))
            .collect::<Vec<_>>()
            .join("")
    }
}

fn deserialize_optional_i64_from_string_or_number<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("url_expire_time must be an i64"))
            .map(Some),
        Some(serde_json::Value::String(value)) => value
            .parse::<i64>()
            .map(Some)
            .map_err(|err| serde::de::Error::custom(format!("invalid url_expire_time: {err}"))),
        Some(other) => Err(serde::de::Error::custom(format!(
            "unsupported url_expire_time value: {other}"
        ))),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(try_from = "u8", into = "u8")]
pub enum TaskStatus {
    Running = 1,
    Success = 2,
    Failure = 3,
}

impl TryFrom<u8> for TaskStatus {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Running),
            2 => Ok(Self::Success),
            3 => Ok(Self::Failure),
            _ => Err("invalid task status"),
        }
    }
}

impl From<TaskStatus> for u8 {
    fn from(value: TaskStatus) -> Self {
        match value {
            TaskStatus::Running => 1,
            TaskStatus::Success => 2,
            TaskStatus::Failure => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_request_serializes_nested_req_params() {
        let req = AsyncSubmitRequest {
            user: AsyncUser { uid: "u1".into() },
            namespace: "BidirectionalTTSAsync".into(),
            unique_id: Some("uniq".into()),
            req_params: AsyncSubmitReqParams {
                text: Some("hello".into()),
                ssml: None,
                speaker: "spk".into(),
                audio_params: AudioParams {
                    format: "mp3".into(),
                    sample_rate: 24_000,
                    speech_rate: Some(0),
                    emotion: Some("sad".into()),
                    loudness_rate: Some(1),
                },
                additions: Some(serde_json::json!({"disable_markdown_filter": true})),
                model: None,
                mix_speaker: None,
            },
        };

        let json = serde_json::to_value(req).unwrap();
        assert_eq!(json["req_params"]["speaker"], "spk");
        assert_eq!(json["req_params"]["audio_params"]["format"], "mp3");
        assert_eq!(json["req_params"]["text"], "hello");
    }

    #[test]
    fn query_response_deserializes_running_success_and_failure() {
        let running: AsyncQueryResponse = serde_json::from_str(r#"{"code":0,"message":"ok","data":{"task_id":"t1","task_status":1,"audio_url":null,"sentences":null,"req_text_length":3,"synthesize_text_length":0,"url_expire_time":null}}"#).unwrap();
        assert_eq!(running.data.task_status, TaskStatus::Running);

        let success: AsyncQueryResponse = serde_json::from_str(r#"{"code":0,"message":"ok","data":{"task_id":"t2","task_status":2,"audio_url":"https://example.com/a.mp3","sentences":[{"text":"hi","startTime":0.0,"endTime":1.0,"words":[{"word":"hi","startTime":0.0,"endTime":1.0,"confidence":0.9}]}],"req_text_length":2,"synthesize_text_length":2,"url_expire_time":"1735689600"}}"#).unwrap();
        assert_eq!(success.data.task_status, TaskStatus::Success);
        assert_eq!(success.data.sentences.as_ref().unwrap()[0].words[0].confidence, 0.9);
        assert_eq!(success.data.url_expire_time, Some(1_735_689_600));

        let failure: AsyncQueryResponse = serde_json::from_str(r#"{"code":0,"message":"ok","data":{"task_id":"t3","task_status":3,"audio_url":null,"sentences":null,"req_text_length":2,"synthesize_text_length":0,"url_expire_time":null}}"#).unwrap();
        assert_eq!(failure.data.task_status, TaskStatus::Failure);
    }

    #[test]
    fn sentences_parse_word_timestamps() {
        let response: AsyncQueryResponse = serde_json::from_str(r#"{"code":0,"message":"ok","data":{"task_id":"t2","task_status":2,"audio_url":"https://example.com/a.mp3","sentences":[{"text":"可以","startTime":0.315,"endTime":2.545,"words":[{"word":"可","startTime":0.315,"endTime":0.455,"confidence":0.8},{"word":"以","startTime":0.455,"endTime":0.600,"confidence":0.82}]}],"req_text_length":2,"synthesize_text_length":2,"url_expire_time":1735689600}}"#).unwrap();

        let sentence = &response.data.sentences.unwrap()[0];
        assert_eq!(sentence.words[0].word, "可");
        assert!((sentence.words[0].start_time - 0.315).abs() < f64::EPSILON);
        assert!((sentence.words[0].confidence - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn sentence_duration_calculated() {
        let sentence = Sentence {
            text: "可以".into(),
            start_time: 0.315,
            end_time: 2.545,
            words: vec![],
        };

        assert!((sentence.duration() - 2.23).abs() < 1e-9);
    }

    #[test]
    fn sentence_text_with_timestamps_formats_words() {
        let sentence = Sentence {
            text: "可以".into(),
            start_time: 0.315,
            end_time: 0.600,
            words: vec![
                Word {
                    word: "可".into(),
                    start_time: 0.315,
                    end_time: 0.455,
                    confidence: 0.8,
                },
                Word {
                    word: "以".into(),
                    start_time: 0.455,
                    end_time: 0.600,
                    confidence: 0.82,
                },
            ],
        };

        assert_eq!(sentence.text_with_timestamps(), "可(0.315-0.455)以(0.455-0.600)");
    }
}

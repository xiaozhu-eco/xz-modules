use crate::error::XzTtsError;
use crate::protocol::EVT_START_SESSION;
use crate::types::TtsSessionConfig;
use serde_json::json;

#[cfg(feature = "voice-mix")]
use crate::types::MixSpeaker;

pub fn build_start_session_json(
    voice_id: &str,
    config: &TtsSessionConfig,
) -> Result<serde_json::Value, XzTtsError> {
    let speaker = if voice_id.is_empty() { &config.voice_id } else { voice_id };
    if speaker.is_empty() {
        return Err(XzTtsError::Config { message: "voice_id is required".into() });
    }
    if !(8000..=48000).contains(&config.sample_rate) {
        return Err(XzTtsError::Config {
            message: format!("sample_rate {} out of range [8000, 48000]", config.sample_rate),
        });
    }

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

    let mut additions = json!({ "disable_markdown_filter": config.disable_markdown_filter });
    if let Some(pitch) = config.pitch {
        additions["post_process"] = json!({ "pitch": pitch });
    }

    let mut req_params = json!({
        "speaker": speaker,
        "audio_params": audio_params,
        "additions": additions.to_string(),
    });
    if let Some(ref context) = config.context_text {
        if !context.is_empty() {
            req_params["context_texts"] = json!([context]);
        }
    }
    if !config.voice_commands.is_empty() {
        req_params["commands"] = json!(config.voice_commands);
    }

    Ok(json!({
        "event": EVT_START_SESSION,
        "namespace": "BidirectionalTTS",
        "req_params": req_params,
    }))
}

#[cfg(feature = "voice-mix")]
pub fn build_mix_start_session_json(
    mix_speakers: &[MixSpeaker],
    config: &TtsSessionConfig,
) -> Result<serde_json::Value, XzTtsError> {
    if mix_speakers.is_empty() {
        return build_start_session_json("", config);
    }
    if mix_speakers.len() > 3 {
        return Err(XzTtsError::Config { message: "max 3 mix speakers".into() });
    }

    let sum: f32 = mix_speakers.iter().map(|s| s.mix_factor).sum();
    if (sum - 1.0).abs() > 0.01 {
        return Err(XzTtsError::Config {
            message: format!("mix factors sum to {}, must be 1.0", sum),
        });
    }

    let mut json = build_start_session_json("custom_mix_bigtts", config)?;
    json["req_params"]["mix_speaker"] = json!({
        "speakers": mix_speakers
            .iter()
            .map(|speaker| json!({
                "source_speaker": speaker.source_speaker,
                "mix_factor": speaker.mix_factor,
            }))
            .collect::<Vec<_>>(),
    });
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudioFormat, TtsSessionConfig};

    #[cfg(feature = "voice-mix")]
    use crate::types::MixSpeaker;

    #[test]
    fn full_config_produces_complete_json() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            emotion_tag: Some("sad".into()),
            speech_rate: Some(0),
            loudness_rate: Some(0),
            pitch: Some(0),
            context_text: Some("hello".into()),
            voice_commands: vec!["cmd1".into()],
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            sample_rate: 24000,
            format: AudioFormat { sample_rate: 24000, channels: 1 },
            disable_markdown_filter: false,
        };
        let json = build_start_session_json("", &config).unwrap();
        let req = &json["req_params"];
        assert_eq!(req["speaker"], "zh_female_xiaohe_uranus_bigtts");
        assert_eq!(req["audio_params"]["emotion"], "sad");
        assert_eq!(req["context_texts"][0], "hello");
        assert_eq!(req["commands"][0], "cmd1");
    }

    #[test]
    fn minimal_config_omits_optional_fields() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            sample_rate: 24000,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            format: AudioFormat { sample_rate: 24000, channels: 1 },
            ..Default::default()
        };
        let json = build_start_session_json("", &config).unwrap();
        let req = &json["req_params"];
        assert_eq!(req["speaker"], "zh_female_xiaohe_uranus_bigtts");
        assert!(req.get("emotion").is_none());
    }

    #[test]
    fn empty_voice_id_returns_error() {
        let config = TtsSessionConfig::default();
        assert!(build_start_session_json("", &config).is_err());
    }

    #[test]
    fn invalid_sample_rate_returns_error() {
        let config = TtsSessionConfig {
            voice_id: "test".into(),
            sample_rate: 100,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            format: AudioFormat { sample_rate: 100, channels: 1 },
            ..Default::default()
        };
        assert!(build_start_session_json("", &config).is_err());
    }

    #[cfg(feature = "voice-mix")]
    #[test]
    fn mix_session_accepts_two_speakers_when_sum_is_one() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            sample_rate: 24000,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            format: AudioFormat { sample_rate: 24000, channels: 1 },
            ..Default::default()
        };
        let mix_speakers = vec![
            MixSpeaker { source_speaker: "speaker_a".into(), mix_factor: 0.6 },
            MixSpeaker { source_speaker: "speaker_b".into(), mix_factor: 0.4 },
        ];

        let json = build_mix_start_session_json(&mix_speakers, &config).unwrap();
        assert_eq!(json["req_params"]["speaker"], "custom_mix_bigtts");
        assert_eq!(json["req_params"]["mix_speaker"]["speakers"].as_array().unwrap().len(), 2);
    }

    #[cfg(feature = "voice-mix")]
    #[test]
    fn mix_session_accepts_three_speakers_when_sum_is_one() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            sample_rate: 24000,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            format: AudioFormat { sample_rate: 24000, channels: 1 },
            ..Default::default()
        };
        let mix_speakers = vec![
            MixSpeaker { source_speaker: "speaker_a".into(), mix_factor: 0.2 },
            MixSpeaker { source_speaker: "speaker_b".into(), mix_factor: 0.3 },
            MixSpeaker { source_speaker: "speaker_c".into(), mix_factor: 0.5 },
        ];

        let json = build_mix_start_session_json(&mix_speakers, &config).unwrap();
        assert_eq!(json["req_params"]["speaker"], "custom_mix_bigtts");
        assert_eq!(json["req_params"]["mix_speaker"]["speakers"].as_array().unwrap().len(), 3);
    }

    #[cfg(feature = "voice-mix")]
    #[test]
    fn mix_session_rejects_invalid_factor_sum() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            sample_rate: 24000,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            format: AudioFormat { sample_rate: 24000, channels: 1 },
            ..Default::default()
        };
        let mix_speakers = vec![
            MixSpeaker { source_speaker: "speaker_a".into(), mix_factor: 0.6 },
            MixSpeaker { source_speaker: "speaker_b".into(), mix_factor: 0.3 },
        ];

        assert!(build_mix_start_session_json(&mix_speakers, &config).is_err());
    }

    #[cfg(feature = "voice-mix")]
    #[test]
    fn mix_session_rejects_more_than_three_speakers() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            sample_rate: 24000,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            format: AudioFormat { sample_rate: 24000, channels: 1 },
            ..Default::default()
        };
        let mix_speakers = vec![
            MixSpeaker { source_speaker: "speaker_a".into(), mix_factor: 0.25 },
            MixSpeaker { source_speaker: "speaker_b".into(), mix_factor: 0.25 },
            MixSpeaker { source_speaker: "speaker_c".into(), mix_factor: 0.25 },
            MixSpeaker { source_speaker: "speaker_d".into(), mix_factor: 0.25 },
        ];

        assert!(build_mix_start_session_json(&mix_speakers, &config).is_err());
    }
}

use crate::error::XzTtsError;
use crate::protocol::EVT_START_SESSION;
use crate::types::TtsSessionConfig;
use serde_json::json;

#[cfg(feature = "voice-mix")]
use crate::types::MixSpeaker;

fn build_additions_json(config: &TtsSessionConfig) -> Result<serde_json::Value, XzTtsError> {
    if let Some(emotion_scale) = config.emotion_scale {
        if !(1..=5).contains(&emotion_scale) {
            return Err(XzTtsError::Config {
                message: format!("emotion_scale {} out of range [1, 5]", emotion_scale),
            });
        }
    }
    if let Some(silence_duration) = config.silence_duration {
        if silence_duration > 30_000 {
            return Err(XzTtsError::Config {
                message: format!("silence_duration {} out of range [0, 30000]", silence_duration),
            });
        }
    }
    if let Some(threshold) = config.unsupported_char_ratio_thresh {
        if !(0.0..=1.0).contains(&threshold) || threshold.is_nan() {
            return Err(XzTtsError::Config {
                message: format!("unsupported_char_ratio_thresh {} out of range [0.0, 1.0]", threshold),
            });
        }
    }

    let mut additions = json!({ "disable_markdown_filter": config.disable_markdown_filter });

    if let Some(pitch) = config.pitch {
        additions["post_process"] = json!({ "pitch": pitch });
    }
    if let Some(value) = config.enable_language_detector {
        additions["enable_language_detector"] = json!(value);
    }
    if let Some(value) = config.explicit_language.as_ref() {
        if !value.is_empty() {
            additions["explicit_language"] = json!(value);
        }
    }
    if let Some(value) = config.context_language.as_ref() {
        if !value.is_empty() {
            additions["context_language"] = json!(value);
        }
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
    if let Some(value) = config.mute_cut_threshold.as_ref() {
        if !value.is_empty() {
            additions["mute_cut_threshold"] = json!(value);
        }
    }
    if let Some(value) = config.mute_cut_remain_ms.as_ref() {
        if !value.is_empty() {
            additions["mute_cut_remain_ms"] = json!(value);
        }
    }

    Ok(additions)
}

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
        "format": config.output_format.to_string(),
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
    if let Some(enable_timestamp) = config.enable_timestamp {
        audio_params["enable_timestamp"] = json!(enable_timestamp);
    }

    let additions = build_additions_json(config)?;

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
    use crate::types::{AudioFormat, TtsOutputFormat, TtsSessionConfig};

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
            emotion_scale: None,
            enable_timestamp: None,
            silence_duration: None,
            enable_language_detector: None,
            explicit_language: None,
            context_language: None,
            unsupported_char_ratio_thresh: None,
            aigc_watermark: None,
            enable_latex_tn: None,
            mute_cut_threshold: None,
            mute_cut_remain_ms: None,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            sample_rate: 24000,
            output_format: TtsOutputFormat::Pcm,
            format: AudioFormat { sample_rate: 24000, channels: 1, output_format: TtsOutputFormat::Pcm },
            disable_markdown_filter: true,
        };
        let json = build_start_session_json("", &config).unwrap();
        let req = &json["req_params"];
        assert_eq!(req["speaker"], "zh_female_xiaohe_uranus_bigtts");
        assert_eq!(req["audio_params"]["emotion"], "sad");
        assert_eq!(req["context_texts"][0], "hello");
        assert_eq!(req["commands"][0], "cmd1");
    }

    #[test]
    fn full_additions_json_emits_all_fields() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            pitch: Some(8),
            voice_commands: vec![],
            emotion_scale: Some(5),
            enable_timestamp: Some(true),
            silence_duration: Some(30000),
            enable_language_detector: Some(true),
            explicit_language: Some("zh-cn".into()),
            context_language: Some("es".into()),
            unsupported_char_ratio_thresh: Some(0.3),
            aigc_watermark: Some(false),
            enable_latex_tn: Some(true),
            mute_cut_threshold: Some("0.15".into()),
            mute_cut_remain_ms: Some("300".into()),
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            sample_rate: 24_000,
            output_format: TtsOutputFormat::Mp3,
            format: AudioFormat { sample_rate: 24_000, channels: 1, output_format: TtsOutputFormat::Mp3 },
            disable_markdown_filter: false,
            ..Default::default()
        };

        let json = build_start_session_json("voice-x", &config).unwrap();
        let req = &json["req_params"];
        let audio_params = &req["audio_params"];
        let additions: serde_json::Value = serde_json::from_str(req["additions"].as_str().unwrap()).unwrap();

        assert_eq!(audio_params["format"], "mp3");
        assert_eq!(audio_params["enable_timestamp"], true);
        assert_eq!(additions["disable_markdown_filter"], false);
        assert_eq!(additions["post_process"]["pitch"], 8);
        assert_eq!(additions["enable_language_detector"], true);
        assert_eq!(additions["explicit_language"], "zh-cn");
        assert_eq!(additions["context_language"], "es");
        assert!((additions["unsupported_char_ratio_thresh"].as_f64().unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(additions["aigc_watermark"], false);
        assert_eq!(additions["enable_latex_tn"], true);
        assert_eq!(additions["mute_cut_threshold"], "0.15");
        assert_eq!(additions["mute_cut_remain_ms"], "300");
        assert!(additions.get("emotion_scale").is_none());
    }

    #[test]
    fn emotion_scale_out_of_range() {
        let config = TtsSessionConfig {
            voice_id: "test".into(),
            emotion_scale: Some(6),
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            sample_rate: 24_000,
            output_format: TtsOutputFormat::Pcm,
            format: AudioFormat { sample_rate: 24_000, channels: 1, output_format: TtsOutputFormat::Pcm },
            ..Default::default()
        };

        assert!(build_start_session_json("", &config).is_err());
    }

    #[test]
    fn silence_duration_out_of_range() {
        let config = TtsSessionConfig {
            voice_id: "test".into(),
            silence_duration: Some(30_001),
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            sample_rate: 24_000,
            output_format: TtsOutputFormat::Pcm,
            format: AudioFormat { sample_rate: 24_000, channels: 1, output_format: TtsOutputFormat::Pcm },
            ..Default::default()
        };

        assert!(build_start_session_json("", &config).is_err());
    }

    #[test]
    fn format_mp3_sets_correct_string() {
        let config = TtsSessionConfig {
            voice_id: "test".into(),
            sample_rate: 24_000,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            output_format: TtsOutputFormat::Mp3,
            format: AudioFormat { sample_rate: 24_000, channels: 1, output_format: TtsOutputFormat::Mp3 },
            ..Default::default()
        };

        let json = build_start_session_json("", &config).unwrap();
        assert_eq!(json["req_params"]["audio_params"]["format"], "mp3");
    }

    #[test]
    fn minimal_config_omits_optional_fields() {
        let config = TtsSessionConfig {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            sample_rate: 24000,
            #[cfg(feature = "voice-mix")]
            mix_speakers: vec![],
            format: AudioFormat { sample_rate: 24000, channels: 1, output_format: TtsOutputFormat::Pcm },
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
            format: AudioFormat { sample_rate: 100, channels: 1, output_format: TtsOutputFormat::Pcm },
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
            format: AudioFormat { sample_rate: 24000, channels: 1, output_format: TtsOutputFormat::Pcm },
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
            format: AudioFormat { sample_rate: 24000, channels: 1, output_format: TtsOutputFormat::Pcm },
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
            format: AudioFormat { sample_rate: 24000, channels: 1, output_format: TtsOutputFormat::Pcm },
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
            format: AudioFormat { sample_rate: 24000, channels: 1, output_format: TtsOutputFormat::Pcm },
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

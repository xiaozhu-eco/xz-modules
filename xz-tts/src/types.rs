#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsOutputFormat {
    /// Raw PCM audio.
    Pcm,
    /// MP3 encoded audio.
    Mp3,
    /// WAV encoded audio.
    Wav,
    /// Ogg Opus encoded audio.
    OggOpus,
}

impl std::fmt::Display for TtsOutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pcm => "pcm",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::OggOpus => "ogg_opus",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub output_format: TtsOutputFormat,
}

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub format: AudioFormat,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TtsVoiceInfo {
    pub voice_id: String,
    pub name: String,
    pub gender: Option<String>,
    pub language: String,
    pub styles: Vec<String>,
    pub preview_url: Option<String>,
    pub scenarios: Vec<String>,
    pub model_version: String,
}

#[cfg(feature = "voice-mix")]
#[derive(Debug, Clone)]
pub struct MixSpeaker {
    pub source_speaker: String,
    pub mix_factor: f32,
}

#[derive(Debug, Clone)]
pub struct TtsSessionConfig {
    pub voice_id: String,
    pub emotion_tag: Option<String>,
    pub speech_rate: Option<i32>,
    pub loudness_rate: Option<i32>,
    pub pitch: Option<i32>,
    pub context_text: Option<String>,
    pub voice_commands: Vec<String>,
    /// ASYNC-ONLY: emotion intensity 1-5.
    pub emotion_scale: Option<u8>,
    /// BOTH: enable word or phoneme timestamps.
    pub enable_timestamp: Option<bool>,
    /// BOTH: silence duration in milliseconds (0-30000).
    pub silence_duration: Option<u32>,
    /// BOTH: enable automatic language detection.
    pub enable_language_detector: Option<bool>,
    /// BOTH: explicit language code (e.g. zh-cn, en, es-mx).
    pub explicit_language: Option<String>,
    /// BOTH: context language for Western European languages.
    pub context_language: Option<String>,
    /// BOTH: unsupported character ratio threshold.
    pub unsupported_char_ratio_thresh: Option<f32>,
    /// BOTH: enable AIGC watermark.
    pub aigc_watermark: Option<bool>,
    /// BOTH: enable LaTeX reading.
    pub enable_latex_tn: Option<bool>,
    /// BOTH: mute cut threshold as API string.
    pub mute_cut_threshold: Option<String>,
    /// BOTH: mute cut remain ms as API string.
    pub mute_cut_remain_ms: Option<String>,
    #[cfg(feature = "voice-mix")]
    pub mix_speakers: Vec<MixSpeaker>,
    pub sample_rate: u32,
    pub output_format: TtsOutputFormat,
    pub format: AudioFormat,
    pub disable_markdown_filter: bool,
}

impl Default for TtsSessionConfig {
    fn default() -> Self {
        Self {
            voice_id: String::new(),
            emotion_tag: None,
            speech_rate: None,
            loudness_rate: None,
            pitch: None,
            context_text: None,
            voice_commands: Vec::new(),
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
            mix_speakers: Vec::new(),
            sample_rate: 24_000,
            output_format: TtsOutputFormat::Pcm,
            format: AudioFormat {
                sample_rate: 24_000,
                channels: 1,
                output_format: TtsOutputFormat::Pcm,
            },
            disable_markdown_filter: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_enum_display_matches_api_strings() {
        assert_eq!(TtsOutputFormat::Pcm.to_string(), "pcm");
        assert_eq!(TtsOutputFormat::Mp3.to_string(), "mp3");
        assert_eq!(TtsOutputFormat::Wav.to_string(), "wav");
        assert_eq!(TtsOutputFormat::OggOpus.to_string(), "ogg_opus");
    }

    #[test]
    fn audio_frame_construction_uses_100_samples_at_24khz() {
        let frame = AudioFrame {
            samples: vec![0.25; 100],
            format: AudioFormat {
                sample_rate: 24_000,
                channels: 1,
                output_format: TtsOutputFormat::Pcm,
            },
            timestamp_ms: 1234,
        };

        assert_eq!(frame.samples.len(), 100);
        assert_eq!(frame.format.sample_rate, 24_000);
        assert_eq!(frame.format.channels, 1);
        assert_eq!(frame.timestamp_ms, 1234);
    }

    #[test]
    fn tts_voice_info_construction_carries_all_fields() {
        let info = TtsVoiceInfo {
            voice_id: "zh_female_xiaohe_uranus_bigtts".into(),
            name: "小何 2.0".into(),
            gender: Some("female".into()),
            language: "zh".into(),
            styles: vec!["happy".into(), "sad".into()],
            preview_url: Some("https://example.com/preview.mp3".into()),
            scenarios: vec!["有声阅读".into(), "客服场景".into()],
            model_version: "2.0".into(),
        };

        assert_eq!(info.voice_id, "zh_female_xiaohe_uranus_bigtts");
        assert_eq!(info.name, "小何 2.0");
        assert_eq!(info.gender.as_deref(), Some("female"));
        assert_eq!(info.language, "zh");
        assert_eq!(info.styles, vec!["happy", "sad"]);
        assert_eq!(info.preview_url.as_deref(), Some("https://example.com/preview.mp3"));
        assert_eq!(info.scenarios, vec!["有声阅读", "客服场景"]);
        assert_eq!(info.model_version, "2.0");
    }

    #[test]
    fn tts_session_config_default_construction_is_available() {
        let config = TtsSessionConfig::default();

        assert_eq!(config.voice_id, "");
        assert_eq!(config.emotion_tag, None);
        assert_eq!(config.speech_rate, None);
        assert_eq!(config.loudness_rate, None);
        assert_eq!(config.pitch, None);
        assert_eq!(config.context_text, None);
        assert!(config.voice_commands.is_empty());
        assert_eq!(config.sample_rate, 24_000);
        assert_eq!(config.output_format, TtsOutputFormat::Pcm);
        assert_eq!(config.format.sample_rate, 24_000);
        assert_eq!(config.format.channels, 1);
        assert_eq!(config.format.output_format, TtsOutputFormat::Pcm);
        assert!(config.disable_markdown_filter);
    }

    #[test]
    fn new_fields_default_to_none() {
        let config = TtsSessionConfig::default();

        assert_eq!(config.emotion_scale, None);
        assert_eq!(config.enable_timestamp, None);
        assert_eq!(config.silence_duration, None);
        assert_eq!(config.enable_language_detector, None);
        assert_eq!(config.explicit_language, None);
        assert_eq!(config.context_language, None);
        assert_eq!(config.unsupported_char_ratio_thresh, None);
        assert_eq!(config.aigc_watermark, None);
        assert_eq!(config.enable_latex_tn, None);
        assert_eq!(config.mute_cut_threshold, None);
        assert_eq!(config.mute_cut_remain_ms, None);
    }

    #[cfg(feature = "voice-mix")]
    #[test]
    fn mix_speaker_is_available_only_with_feature() {
        let mix = MixSpeaker {
            source_speaker: "zh_female_vv_uranus_bigtts".into(),
            mix_factor: 0.7,
        };

        assert_eq!(mix.source_speaker, "zh_female_vv_uranus_bigtts");
        assert_eq!(mix.mix_factor, 0.7);
    }
}

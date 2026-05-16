use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;

use crate::error::XzTtsError;

#[async_trait]
pub trait TextPreprocessor: Send + Sync {
    /// Preprocess text before TTS synthesis.
    /// Returns the processed text.
    async fn preprocess(&self, text: &str) -> Result<String, XzTtsError>;
}

pub struct NoOpPreprocessor;

#[async_trait]
impl TextPreprocessor for NoOpPreprocessor {
    async fn preprocess(&self, text: &str) -> Result<String, XzTtsError> {
        Ok(text.to_string())
    }
}

pub struct VoiceCommandExtractor;

impl VoiceCommandExtractor {
    /// Extract voice commands from text using `[#command]` syntax.
    /// Returns (cleaned_text, commands).
    pub fn extract_commands(text: &str) -> (String, Vec<String>) {
        let regex = command_regex();
        const ESCAPED_OPEN: &str = "__XZ_ESCAPED_OPEN__";
        const ESCAPED_CLOSE: &str = "__XZ_ESCAPED_CLOSE__";

        let escaped = text
            .replace(r"\[#", ESCAPED_OPEN)
            .replace(r"\]", ESCAPED_CLOSE);

        let commands = regex
            .captures_iter(&escaped)
            .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .collect::<Vec<_>>();

        let cleaned = regex
            .replace_all(&escaped, "")
            .replace(ESCAPED_OPEN, "[#")
            .replace(ESCAPED_CLOSE, "]");

        (cleaned, commands)
    }
}

fn command_regex() -> &'static Regex {
    static COMMAND_REGEX: OnceLock<Regex> = OnceLock::new();
    COMMAND_REGEX.get_or_init(|| Regex::new(r"\[#([^\]]+)\]").expect("valid command regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_returns_same_text() {
        let preprocessor = NoOpPreprocessor;
        let input = "你好 world";

        let output = preprocessor.preprocess(input).await.unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn extract_single_command() {
        let (cleaned, commands) = VoiceCommandExtractor::extract_commands("你好 [#用悲伤的语气说] 世界");

        assert_eq!(cleaned, "你好  世界");
        assert_eq!(commands, vec!["用悲伤的语气说"]);
    }

    #[test]
    fn extract_multiple_commands() {
        let (cleaned, commands) = VoiceCommandExtractor::extract_commands("[ #ignore ] 这里不算 [#快一点] 还有 [#慢一点]");

        assert_eq!(cleaned, "[ #ignore ] 这里不算  还有 ");
        assert_eq!(commands, vec!["快一点", "慢一点"]);
    }

    #[test]
    fn no_commands_in_plain_text() {
        let (cleaned, commands) = VoiceCommandExtractor::extract_commands("Hello world");

        assert_eq!(cleaned, "Hello world");
        assert!(commands.is_empty());
    }

    #[test]
    fn escaped_bracket_preserved() {
        let (cleaned, commands) = VoiceCommandExtractor::extract_commands("This is \\[#literal\\] text and [#command]");

        assert_eq!(cleaned, "This is [#literal] text and ");
        assert_eq!(commands, vec!["command"]);
    }

    #[test]
    fn empty_text() {
        let (cleaned, commands) = VoiceCommandExtractor::extract_commands("");

        assert_eq!(cleaned, "");
        assert!(commands.is_empty());
    }
}

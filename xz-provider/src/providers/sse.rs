/// SSE line accumulator that buffers raw bytes across HTTP chunk boundaries.
///
/// When SSE events are split across byte chunks, this buffer retains partial
/// lines until a `\n` terminator arrives. Only complete lines are yielded.
pub(crate) struct SseLineBuffer {
    buffer: Vec<u8>,
}

impl SseLineBuffer {
    pub(crate) fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Feed a chunk of raw bytes. Returns all complete lines found.
    ///
    /// Any trailing partial line stays in the buffer and will be completed
    /// when the next chunk arrives.
    pub(crate) fn feed(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut lines = Vec::new();
        let mut drain_end = 0;

        for (i, &b) in self.buffer.iter().enumerate() {
            if b == b'\n' {
                let line_bytes = &self.buffer[drain_end..i];
                let line_bytes = line_bytes.strip_suffix(b"\r").unwrap_or(line_bytes);
                let line = String::from_utf8_lossy(line_bytes).into_owned();
                lines.push(line);
                drain_end = i + 1;
            }
        }

        self.buffer.drain(..drain_end);
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_chunk_returns_empty() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_single_complete_line() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"hello\n");
        assert_eq!(lines, vec!["hello"]);
    }

    #[test]
    fn test_multiple_complete_lines() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"line1\nline2\nline3\n");
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_crlf_line_endings() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"line1\r\nline2\r\n");
        assert_eq!(lines, vec!["line1", "line2"]);
    }

    #[test]
    fn test_mixed_crlf_and_lf() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"line1\r\nline2\nline3\r\n");
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_no_trailing_newline_retains_bytes() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"incomplete");
        assert!(lines.is_empty());
        assert_eq!(buf.buffer, b"incomplete");
    }

    #[test]
    fn test_sse_fragmented_chunks_line_boundary() {
        let mut buf = SseLineBuffer::new();
        let lines1 = buf.feed(b"left-");
        assert!(lines1.is_empty(), "nothing complete yet");

        let lines2 = buf.feed(b"right\n");
        assert_eq!(lines2, vec!["left-right"], "line reassembled");
    }

    #[test]
    fn test_sse_fragmented_chunks_multi_line() {
        let mut buf = SseLineBuffer::new();
        let lines1 = buf.feed(b"line1\nli");
        assert_eq!(lines1, vec!["line1"]);

        let lines2 = buf.feed(b"ne2\nline3\n");
        assert_eq!(lines2, vec!["line2", "line3"]);
    }

    #[test]
    fn test_sse_fragmented_chunks_byte_by_byte() {
        let mut buf = SseLineBuffer::new();
        let input = b"a\nbb\nccc\n";
        let mut all_lines = Vec::new();
        for &byte in input {
            let lines = buf.feed(&[byte]);
            all_lines.extend(lines);
        }
        assert_eq!(all_lines, vec!["a", "bb", "ccc"]);
    }

    #[test]
    fn test_sse_fragmented_chunks_random_split() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let mut rng = SimpleRng::new(seed);

        let body = b"event: message_start\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello World\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" How are you?\"}}\n\n";

        let mut chunks = Vec::new();
        let mut pos = 0;
        while pos < body.len() {
            let chunk_size = (rng.next() as usize % 20).max(1).min(body.len() - pos);
            chunks.push(&body[pos..pos + chunk_size]);
            pos += chunk_size;
        }

        let mut buf = SseLineBuffer::new();
        let mut all_lines = Vec::new();
        for chunk in chunks {
            let lines = buf.feed(chunk);
            all_lines.extend(lines);
        }

        let expected_lines: Vec<&str> = vec![
            "event: message_start",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello World\"}}",
            "",
            "event: content_block_delta",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" How are you?\"}}",
            "",
        ];
        assert_eq!(all_lines, expected_lines);
    }

    #[test]
    fn test_sse_fragmented_chunks_data_split_across_chunks() {
        let mut buf = SseLineBuffer::new();

        let lines1 = buf.feed(b"data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"He");
        assert!(lines1.is_empty(), "no complete line yet");

        let lines2 = buf.feed(b"llo\"}}\n");
        assert_eq!(
            lines2,
            vec!["data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}"]
        );
    }

    #[test]
    fn test_sse_utf8_chinese_characters_split_across_chunks() {
        // "你好世界" byte layout: e4 bd a0 | e5 a5 bd | e4 b8 96 | e7 95 8c
        let mut buf = SseLineBuffer::new();

        let lines1 = buf.feed(b"data: \xe4\xbd\xa0\xe5");
        assert!(lines1.is_empty());

        let lines2 = buf.feed(b"\xa5\xbd\xe4\xb8\x96\xe7\x95\x8c\n");
        assert_eq!(lines2.len(), 1);
        assert!(lines2[0].contains("你好"));
    }

    #[test]
    fn test_sse_utf8_emoji_split_across_chunks() {
        // 🌍 = f0 9f 8c 8d (4-byte UTF-8)
        let mut buf = SseLineBuffer::new();

        let lines1 = buf.feed(b"data: Hello \xf0\x9f");
        assert!(lines1.is_empty());

        let lines2 = buf.feed(b"\x8c\x8d\n");
        assert_eq!(lines2.len(), 1);
        assert!(lines2[0].contains("Hello"));
    }

    #[test]
    fn test_sse_utf8_complete_multibyte_across_chunks() {
        let mut buf = SseLineBuffer::new();
        // "café" = 63 61 66 c3 a9
        let lines1 = buf.feed(b"data: caf\xc3");
        assert!(lines1.is_empty());

        let lines2 = buf.feed(b"\xa9\n");
        assert_eq!(lines2, vec!["data: café"]);
    }

    #[test]
    fn test_sse_full_claude_event_parsing_random_chunks() {
        let body = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_001\",\"role\":\"assistant\",\"model\":\"claude-3-opus-20240229\",\"content\":[],\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n",
            "\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n",
            "\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n",
            "\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" World\"}}\n",
            "\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":5}}\n",
            "\n",
        );
        let body_bytes = body.as_bytes();

        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let mut rng = SimpleRng::new(seed);

        let mut chunks = Vec::new();
        let mut pos = 0;
        while pos < body_bytes.len() {
            let max_size = (rng.next() as usize % 13).max(3);
            let chunk_size = max_size.min(body_bytes.len() - pos);
            chunks.push(&body_bytes[pos..pos + chunk_size]);
            pos += chunk_size;
        }

        let mut buf = SseLineBuffer::new();
        let mut all_lines = Vec::new();
        for chunk in chunks {
            let lines = buf.feed(chunk);
            all_lines.extend(lines);
        }

        let data_lines: Vec<&str> = all_lines
            .iter()
            .filter(|l| l.starts_with("data: "))
            .map(|l| l.strip_prefix("data: ").unwrap())
            .collect();

        assert_eq!(data_lines.len(), 5, "expected 5 data events");

        for (i, line) in data_lines.iter().enumerate() {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
            assert!(parsed.is_ok(), "line {} is invalid JSON: {}", i, line);
        }

        let first: serde_json::Value = serde_json::from_str(data_lines[0]).unwrap();
        assert_eq!(first["type"], "message_start");

        let second: serde_json::Value = serde_json::from_str(data_lines[1]).unwrap();
        assert_eq!(second["type"], "content_block_start");

        let third: serde_json::Value = serde_json::from_str(data_lines[2]).unwrap();
        assert_eq!(third["delta"]["text"], "Hello");

        let fourth: serde_json::Value = serde_json::from_str(data_lines[3]).unwrap();
        assert_eq!(fourth["delta"]["text"], " World");
    }

    #[test]
    fn test_sse_openai_style_events_random_chunks() {
        let body = concat!(
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"}}]}\n",
            "\n",
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" World\"}}]}\n",
            "\n",
            "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n",
            "\n",
            "data: [DONE]\n",
            "\n",
        );
        let body_bytes = body.as_bytes();

        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let mut rng = SimpleRng::new(seed);

        let mut chunks = Vec::new();
        let mut pos = 0;
        while pos < body_bytes.len() {
            let max_size = (rng.next() as usize % 17).max(2);
            let chunk_size = max_size.min(body_bytes.len() - pos);
            chunks.push(&body_bytes[pos..pos + chunk_size]);
            pos += chunk_size;
        }

        let mut buf = SseLineBuffer::new();
        let mut all_lines = Vec::new();
        for chunk in chunks {
            let lines = buf.feed(chunk);
            all_lines.extend(lines);
        }

        let data_lines: Vec<&str> = all_lines
            .iter()
            .filter(|l| l.starts_with("data: "))
            .map(|l| l.strip_prefix("data: ").unwrap())
            .collect();

        assert!(data_lines.len() >= 4, "expected at least 4 data events");
        assert!(data_lines.contains(&"[DONE]"));
    }

    #[test]
    fn test_sse_ollama_ndjson_fragmented() {
        let body = b"{\"model\":\"llama3\",\"response\":\"Hello\",\"done\":false}\n{\"model\":\"llama3\",\"response\":\" World\",\"done\":false}\n{\"model\":\"llama3\",\"done\":true}\n";

        let mut buf = SseLineBuffer::new();
        let lines1 = buf.feed(&body[..20]);
        let lines2 = buf.feed(&body[20..]);

        let all_lines: Vec<String> = lines1.into_iter().chain(lines2).collect();
        assert_eq!(all_lines.len(), 3, "expected 3 JSON lines");
        for line in &all_lines {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
            assert!(parsed.is_ok(), "invalid JSON: {}", line);
        }
    }

    #[test]
    fn test_sse_empty_lines_preserved() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"data: {\"a\":1}\n\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1], "");
    }

    #[test]
    fn test_sse_multiple_feed_empty_result() {
        let mut buf = SseLineBuffer::new();
        buf.feed(b"partial");
        buf.feed(b"");
        let lines = buf.feed(b" line\n");
        assert_eq!(lines, vec!["partial line"]);
    }

    #[test]
    fn test_sse_buffer_reuse_after_clear() {
        let mut buf = SseLineBuffer::new();
        buf.feed(b"first\n");
        buf.feed(b"second\n");
        let lines = buf.feed(b"third\n");
        assert_eq!(lines, vec!["third"]);
        assert!(buf.buffer.is_empty());
    }

    #[test]
    fn test_sse_consecutive_newlines() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.feed(b"\n\n\n");
        assert_eq!(lines, vec!["", "", ""]);
    }

    /// Verifies that per-chunk `text.lines()` (the old approach) loses SSE
    /// events split across chunk boundaries, while `SseLineBuffer` preserves them.
    /// Expected: pre-fix FAIL (broken_events.len() != 2), post-fix PASS.
    #[test]
    fn test_sse_broken_per_chunk_lines_demo() {
        let body = b"data: {\"delta\":\"Hello\"}\n\ndata: {\"delta\":\"World\"}\n\n";

        let chunk1 = &body[..12];
        let chunk2 = &body[12..];

        let mut broken_events: Vec<String> = Vec::new();
        for chunk in &[chunk1, chunk2] {
            let text = String::from_utf8_lossy(chunk);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    broken_events.push(data.to_string());
                }
            }
        }

        let mut buf = SseLineBuffer::new();
        let fixed_lines: Vec<String> = buf
            .feed(chunk1)
            .into_iter()
            .chain(buf.feed(chunk2))
            .collect();
        let fixed_events: Vec<&str> = fixed_lines
            .iter()
            .filter_map(|l| l.strip_prefix("data: "))
            .collect();

        assert_eq!(fixed_events.len(), 2);
        assert_eq!(fixed_events, vec!["{\"delta\":\"Hello\"}", "{\"delta\":\"World\"}"]);

        assert_ne!(
            broken_events.len(),
            2,
            "old per-chunk lines() approach loses events"
        );
    }

    struct SimpleRng {
        state: u32,
    }

    impl SimpleRng {
        fn new(seed: u32) -> Self {
            Self { state: seed.max(1) }
        }

        fn next(&mut self) -> u32 {
            self.state ^= self.state << 13;
            self.state ^= self.state >> 17;
            self.state ^= self.state << 5;
            self.state
        }
    }
}

/// 关键词提取 prompt 模板
pub const KEYWORD_EXTRACTION_PROMPT: &str = r#"You are a search query optimizer. Given a natural language question,
extract 3-5 most effective search keywords. Output as a JSON array.

Rules:
- Remove stop words, politeness phrases
- Keep technical terms intact
- Use English keywords for non-English queries
- Prioritize nouns and compound terms

Question: {query}
Output: ["keyword1", "keyword2", ...]"#;

/// 多角度表述 prompt 模板
pub const MULTI_PERSPECTIVE_PROMPT: &str = r#"Given a search query, generate {n} alternative phrasings
that could help find relevant information from different angles.
Output as a JSON array of strings.

Original query: {query}
Alternative phrasings:"#;

/// 翻译为英文 prompt 模板
pub const TRANSLATE_TO_ENGLISH_PROMPT: &str = r#"Translate the following search query to English.
Focus on extracting the key search terms, not a literal translation.
Output only the English search keywords, no explanation.

Query: {query}
English keywords:"#;

/// 分解查询 prompt 模板
pub const DECOMPOSE_PROMPT: &str = r#"Break down the following complex question into
at most {max_subqueries} simpler sub-questions that can be searched independently.
Output as a JSON array of strings.

Complex question: {query}
Sub-questions:"#;

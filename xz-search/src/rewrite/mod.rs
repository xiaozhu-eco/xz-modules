pub mod templates;

use std::collections::HashSet;

/// 查询重写器 — 通过启发式规则或 LLM 优化搜索查询
///
/// 使用场景：
/// - 用户输入自然语言 → 提取关键词
/// - 拆分复杂查询为多个子查询
/// - 多角度表述同一查询
#[derive(Debug)]
pub struct QueryRewriter {
    prompt_template: String,
}

/// 预置重写模板
pub enum RewriteTemplate {
    /// 关键词提取
    KeywordExtraction,
    /// 多角度表述
    MultiPerspective { n: usize },
    /// 翻译为英文
    TranslateToEnglish,
    /// 分解查询
    Decompose { max_subqueries: usize },
}

/// 常见停用词（中文 + 英文）
const STOP_WORDS: &[&str] = &[
    "的", "了", "在", "是", "我", "有", "和", "就", "不", "人", "都", "一",
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "can", "shall", "to", "of", "in", "for",
    "on", "with", "at", "by", "from", "as", "into", "about", "what",
    "which", "who", "whom", "this", "that", "these", "those", "it", "its",
    "and", "but", "or", "not", "no", "if", "then", "else", "when",
    "how", "why", "where", "我", "你", "他", "她", "它", "们", "这",
    "那", "哪", "什么", "怎么", "为什么", "哪", "谁", "吗", "吧", "呢",
];

impl QueryRewriter {
    pub fn new(prompt_template: &str) -> Self {
        Self {
            prompt_template: prompt_template.to_string(),
        }
    }

    /// 使用模板重写查询（无 LLM 的启发式实现）
    pub async fn rewrite_with_template(
        &self,
        query: &str,
        template: RewriteTemplate,
    ) -> Result<Vec<String>, crate::error::SearchError> {
        match template {
            RewriteTemplate::KeywordExtraction => Ok(self.extract_keywords(query)),
            RewriteTemplate::MultiPerspective { n } => {
                Ok(self.generate_perspectives(query, n))
            }
            RewriteTemplate::TranslateToEnglish => {
                // 无 LLM 时返回原始查询
                Ok(vec![query.to_string()])
            }
            RewriteTemplate::Decompose { max_subqueries } => {
                Ok(self.decompose_query(query, max_subqueries))
            }
        }
    }

    /// 多角度查询拓展
    pub async fn multi_perspective(
        &self,
        query: &str,
        n: usize,
    ) -> Result<Vec<String>, crate::error::SearchError> {
        Ok(self.generate_perspectives(query, n))
    }

    // ─── 启发式方法 ────────────────────────────────────────────

    /// 从查询中提取关键词（去除停用词和标点）
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let stop_words: HashSet<&str> = STOP_WORDS.iter().copied().collect();

        // 简单的分词：按空格和常见标点分割
        let tokens: Vec<String> = text
            .split(|c: char| c.is_ascii_punctuation() || c.is_whitespace() || c == '，' || c == '。' || c == '？' || c == '！' || c == '、')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s.len() >= 2)
            .collect();

        if tokens.is_empty() {
            return vec![text.to_string()];
        }

        // 英文停用词检查 + 中文短词过滤
        let filtered: Vec<String> = tokens
            .iter()
            .filter(|t| {
                if stop_words.contains(t.as_str()) {
                    return false;
                }
                // 单中文字符通常有意义，保留
                if t.chars().all(|c| c.is_ascii_alphabetic()) && t.len() <= 2 {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        if filtered.is_empty() {
            return vec![text.to_string()];
        }

        // 关键词连接成搜索词
        let keywords = filtered.join(" ");
        if keywords.len() < text.len() / 2 {
            // 如果删太多，回退到原始查询
            vec![text.to_string(), keywords]
        } else {
            vec![keywords]
        }
    }

    /// 生成多角度查询变体
    fn generate_perspectives(&self, query: &str, n: usize) -> Vec<String> {
        let mut results = vec![query.to_string()];

        let modifiers = [
            "best", "top", "guide", "tutorial", "how to",
            "最新", "指南", "教程", "最佳", "推荐",
        ];

        for i in 1..n {
            let modifier = modifiers[i % modifiers.len()];
            results.push(format!("{} {}", query, modifier));
        }

        results
    }

    /// 按连接词分解复杂查询
    fn decompose_query(&self, query: &str, max: usize) -> Vec<String> {
        let separators = [" and ", " or ", " vs ", " 和 ", " 或 ", " 与 ", " 对比 "];

        for sep in &separators {
            if query.contains(sep) {
                let parts: Vec<String> = query
                    .splitn(max + 1, sep)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if parts.len() > 1 {
                    return parts;
                }
            }
        }

        vec![query.to_string()]
    }
}

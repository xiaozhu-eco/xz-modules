use crate::error::AgentError;
use crate::types::step::ReportFormat;

pub fn execute_report(title: &str, format: &ReportFormat) -> Result<String, AgentError> {
    let body = format!("# {}\n\nReport generated at: {}", title, chrono_now());

    match format {
        ReportFormat::Markdown => Ok(body),
        ReportFormat::Html => Ok(format!("<h1>{}</h1><p>Generated at: {}</p>", title, chrono_now())),
        ReportFormat::Json => Ok(serde_json::json!({
            "title": title,
            "generated_at": chrono_now(),
        }).to_string()),
        ReportFormat::PlainText => Ok(format!("{}\n\nGenerated at: {}", title, chrono_now())),
    }
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default()
}

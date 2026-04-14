use crate::models::{ParsedContent, RawMessage};
use anyhow::{Context, Result};
use std::path::Path;

lazy_static::lazy_static! {
    // 常见 QQ 导出头部：张三 2024/03/15 14:32:05
    static ref SENDER_FIRST_HEADER: regex::Regex = regex::Regex::new(
        r"^(.+?)\s+(\d{4}[-/年]\d{1,2}[-/月]\d{1,2}(?:日)?\s+\d{1,2}:\d{2}(?::\d{2})?)$"
    ).unwrap();

    // 兼容少量时间在前的导出格式：2024-03-15 14:32:05 张三
    static ref TIMESTAMP_FIRST_HEADER: regex::Regex = regex::Regex::new(
        r"^(\d{4}[-/年]\d{1,2}[-/月]\d{1,2}(?:日)?\s+\d{1,2}:\d{2}(?::\d{2})?)\s+(.+)$"
    ).unwrap();
}

pub fn parse_txt(path: &Path) -> Result<ParsedContent> {
    let content = std::fs::read_to_string(path).context("Failed to read QQ txt file")?;
    parse_txt_content(&content)
}

pub fn parse_txt_content(text: &str) -> Result<ParsedContent> {
    let mut messages: Vec<RawMessage> = Vec::new();
    let mut current_sender = String::new();
    let mut current_timestamp = String::new();
    let mut current_content = String::new();
    let mut header_found = false;

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if let Some((sender, timestamp)) = parse_header(line) {
            header_found = true;

            if !current_content.trim().is_empty() {
                messages.push(RawMessage {
                    sender: current_sender.clone(),
                    content: current_content.trim().to_string(),
                    timestamp: Some(current_timestamp.clone()),
                    is_from_me: is_self_sender(&current_sender),
                });
            }

            current_sender = sender;
            current_timestamp = normalize_timestamp(&timestamp);
            current_content.clear();
            continue;
        }

        if line.trim().is_empty() {
            continue;
        }

        if !current_content.is_empty() {
            current_content.push('\n');
        }
        current_content.push_str(line.trim());
    }

    if !header_found {
        return Ok(ParsedContent {
            source: "qq_txt".to_string(),
            target_name: None,
            messages: Vec::new(),
            message_count: 0,
        });
    }

    if !current_content.trim().is_empty() {
        let is_from_me = is_self_sender(&current_sender);
        messages.push(RawMessage {
            sender: current_sender,
            content: current_content.trim().to_string(),
            timestamp: Some(current_timestamp),
            is_from_me,
        });
    }

    let count = messages.len();
    Ok(ParsedContent {
        source: "qq_txt".to_string(),
        target_name: None,
        messages,
        message_count: count,
    })
}

pub fn looks_like_qq_txt(text: &str) -> bool {
    text.lines()
        .take(20)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .any(|line| parse_header(line).is_some())
}

fn parse_header(line: &str) -> Option<(String, String)> {
    if let Some(caps) = SENDER_FIRST_HEADER.captures(line) {
        let sender = caps[1].trim().to_string();
        let timestamp = caps[2].trim().to_string();
        if looks_like_sender(&sender) {
            return Some((sender, timestamp));
        }
    }

    if let Some(caps) = TIMESTAMP_FIRST_HEADER.captures(line) {
        let timestamp = caps[1].trim().to_string();
        let sender = caps[2].trim().to_string();
        if looks_like_sender(&sender) {
            return Some((sender, timestamp));
        }
    }

    None
}

fn looks_like_sender(sender: &str) -> bool {
    let trimmed = sender.trim();
    !trimmed.is_empty() && trimmed.len() <= 64 && !trimmed.contains("http")
}

fn normalize_timestamp(timestamp: &str) -> String {
    timestamp
        .replace('年', "-")
        .replace('月', "-")
        .replace('日', "")
        .replace('/', "-")
}

fn is_self_sender(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "我"
        || lower == "me"
        || lower == "self"
        || lower.contains("自己")
        || lower.contains("(我)")
        || lower.contains("（我）")
}

#[cfg(test)]
mod tests {
    use super::{looks_like_qq_txt, parse_txt_content};

    #[test]
    fn parses_sender_first_qq_export() {
        let input = "\
张三 2024/03/15 14:32:05
今天下班一起吃饭吗？

我 2024/03/15 14:33:10
可以，七点见。
";

        let parsed = parse_txt_content(input).expect("parse qq txt");
        assert_eq!(parsed.message_count, 2);
        assert_eq!(parsed.messages[0].sender, "张三");
        assert_eq!(parsed.messages[0].content, "今天下班一起吃饭吗？");
        assert_eq!(parsed.messages[0].timestamp.as_deref(), Some("2024-03-15 14:32:05"));
        assert!(!parsed.messages[0].is_from_me);
        assert!(parsed.messages[1].is_from_me);
    }

    #[test]
    fn returns_empty_when_not_qq_format() {
        let parsed = parse_txt_content("这是一段普通文本\n没有消息头").expect("parse fallback");
        assert_eq!(parsed.message_count, 0);
        assert!(parsed.messages.is_empty());
    }

    #[test]
    fn detects_sender_first_headers() {
        let input = "\
小林 2024/03/15 14:32:05
你好呀
";
        assert!(looks_like_qq_txt(input));
    }
}

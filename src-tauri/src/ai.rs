//! Anthropic Messages API call + parsing of the model's JSON answer.

use crate::store::Draft;
use serde::Deserialize;
use serde_json::json;

/// Default model. Override with ANTHROPIC_MODEL in .env if this name is retired —
/// check the current list at https://docs.claude.com before shipping.
pub const DEFAULT_MODEL: &str = "claude-sonnet-5";
pub const API_URL: &str = "https://api.anthropic.com/v1/messages";
pub const API_VERSION: &str = "2023-06-01";

/// Kept as its own const so it can be reviewed on its own.
pub const SYSTEM_PROMPT: &str = "\
You help a ministry worker prepare to answer hard questions about faith that people have asked them. \
Write a SHORT, humble, pastoral first response (about 3-6 sentences) in plain language. \
Do not overclaim, do not invent quotations, statistics, or references, and be honest about uncertainty. \
Then list 2 to 4 concrete things the worker should verify before answering: specific claims, \
scripture references, historical or scientific facts, or sources to check. \
Your output is only a starting point for a human to review, never a final answer. \
Respond with JSON ONLY, no markdown fences and no other text, in exactly this shape: \
{\"draft\": \"...\", \"verify\": [\"...\", \"...\"]}";

#[derive(Deserialize)]
struct RawDraft {
    draft: String,
    verify: Vec<String>,
}

/// Parse the model's text into a Draft. Tolerates ```json fences and surrounding chatter.
pub fn parse_draft(text: &str, model: &str, created_at: &str) -> Result<Draft, String> {
    let start = text.find('{');
    let end = text.rfind('}');
    let slice = match (start, end) {
        (Some(s), Some(e)) if e > s => &text[s..=e],
        _ => return Err(parse_err()),
    };
    let raw: RawDraft = serde_json::from_str(slice).map_err(|_| parse_err())?;
    let draft = raw.draft.trim().to_string();
    let verify: Vec<String> = raw
        .verify
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .take(4)
        .collect();
    if draft.is_empty() || verify.is_empty() {
        return Err(parse_err());
    }
    Ok(Draft { draft, verify, model: model.to_string(), created_at: created_at.to_string() })
}

fn parse_err() -> String {
    "The AI's reply wasn't in the expected format. Nothing was saved — please try Draft again.".into()
}

/// Ask the model for a draft. `api_key` is passed in and never logged.
pub async fn request_draft(api_key: &str, question_text: &str, context: &str, asker: &str) -> Result<Draft, String> {
    let model = std::env::var("ANTHROPIC_MODEL")
        .ok()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    let mut user_msg = format!("Question: {question_text}");
    if !asker.is_empty() {
        user_msg.push_str(&format!("\nAsked by: {asker}"));
    }
    if !context.is_empty() {
        user_msg.push_str(&format!("\nContext: {context}"));
    }

    let body = json!({
        "model": model,
        "max_tokens": 1024,
        "system": SYSTEM_PROMPT,
        "messages": [{ "role": "user", "content": user_msg }],
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Could not start the network client: {e}"))?;

    let resp = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", API_VERSION)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "The AI service took too long to answer. Check your connection and try again.".to_string()
            } else {
                "Could not reach the AI service. Check your internet connection and try again.".to_string()
            }
        })?;

    let status = resp.status();
    let payload: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| "The AI service sent a reply that could not be read.".to_string())?;

    if !status.is_success() {
        return Err(api_error_message(status.as_u16(), &payload));
    }

    let text = payload["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find_map(|b| b["text"].as_str()))
        .ok_or_else(|| "The AI service returned no text.".to_string())?;

    parse_draft(text, &model, &chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

fn api_error_message(status: u16, payload: &serde_json::Value) -> String {
    let detail = payload["error"]["message"].as_str().unwrap_or("no details");
    match status {
        401 | 403 => "The AI service rejected the API key. Check ANTHROPIC_API_KEY in your .env file.".into(),
        404 => format!("The AI model was not found ({detail}). Set ANTHROPIC_MODEL to a current model name."),
        429 => "The AI service is busy or your usage limit was reached. Wait a moment and try again.".into(),
        _ => format!("The AI service returned an error (HTTP {status}): {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let d = parse_draft(r#"{"draft":"Hello","verify":["a","b"]}"#, "m", "t").unwrap();
        assert_eq!(d.draft, "Hello");
        assert_eq!(d.verify.len(), 2);
    }

    #[test]
    fn parses_fenced_json() {
        let t = "```json\n{\"draft\":\"Hi\",\"verify\":[\"x\"]}\n```";
        assert!(parse_draft(t, "m", "t").is_ok());
    }

    #[test]
    fn caps_verify_at_four() {
        let t = r#"{"draft":"Hi","verify":["1","2","3","4","5","6"]}"#;
        assert_eq!(parse_draft(t, "m", "t").unwrap().verify.len(), 4);
    }

    #[test]
    fn rejects_garbage_and_empty() {
        assert!(parse_draft("sorry, I can't", "m", "t").is_err());
        assert!(parse_draft(r#"{"draft":"","verify":["a"]}"#, "m", "t").is_err());
        assert!(parse_draft(r#"{"draft":"a","verify":[]}"#, "m", "t").is_err());
        assert!(parse_draft(r#"{"draft":"a"}"#, "m", "t").is_err());
    }
}

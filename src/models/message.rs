use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "system" => Some(Self::System),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub tool_results: Option<Value>,
    pub tokens_count: usize,
    pub collapsed_content: Option<String>,
    pub collapsed_tokens_count: usize,
    pub is_indexed: bool,
    pub summary_ref: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub tool_results: Option<Value>,
}

/// Estimate the number of tokens in a text string.
///
/// Uses a simple heuristic: `ceil(char_count / 3.5) + 4`.
/// This is intentionally a stub until a proper tokenizer is integrated.
pub fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();
    ((char_count as f64) / 3.5).ceil() as usize + 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 4);
    }

    #[test]
    fn test_estimate_tokens_short() {
        // "Hola" = 4 chars: ceil(4/3.5)=2 + 4 = 6
        assert_eq!(estimate_tokens("Hola"), 6);
    }

    #[test]
    fn test_estimate_tokens_long() {
        // 3500 chars: ceil(3500/3.5) + 4 = 1000 + 4 = 1004
        let long_text = "a".repeat(3500);
        assert_eq!(estimate_tokens(&long_text), 1004);
    }

    #[test]
    fn test_estimate_tokens_hello_world() {
        // "Hello World" = 11 chars: ceil(11/3.5)=4 + 4 = 8
        assert_eq!(estimate_tokens("Hello World"), 8);
    }
}

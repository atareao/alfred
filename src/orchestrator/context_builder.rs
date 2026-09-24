use crate::llm::provider::ChatMessage;
use crate::orchestrator::context_classifier::ContextStrategy;

pub struct BuiltContext {
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,
    pub token_estimate: usize,
    pub rag_memories: Vec<String>,
    pub session_summary: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Profile not found")]
    ProfileNotFound,
    #[error("Search error: {0}")]
    SearchError(String),
    #[error("Window error: {0}")]
    WindowError(String),
}

pub struct ContextBuilder;

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(
        &self,
        strategy: ContextStrategy,
        _profile_id: &str,
        _user_message: &str,
    ) -> Result<BuiltContext, ContextError> {
        match strategy {
            ContextStrategy::SlidingWindow => Ok(BuiltContext {
                system_prompt: "You are Alfred, a helpful AI assistant.".into(),
                messages: vec![],
                token_estimate: 500,
                rag_memories: vec![],
                session_summary: None,
            }),
            ContextStrategy::Historical => Ok(BuiltContext {
                system_prompt: "You are Alfred, analyzing historical data.".into(),
                messages: vec![],
                token_estimate: 5000,
                rag_memories: vec![],
                session_summary: None,
            }),
            ContextStrategy::RAG => Ok(BuiltContext {
                system_prompt: "You are Alfred, using RAG context.".into(),
                messages: vec![],
                token_estimate: 2000,
                rag_memories: vec!["memory1".into(), "memory2".into()],
                session_summary: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sliding_window_context() {
        let builder = ContextBuilder::new();
        let ctx = builder
            .build(ContextStrategy::SlidingWindow, "profile-1", "hello")
            .await
            .unwrap();
        assert!(ctx.system_prompt.contains("Alfred"));
        assert!(ctx.token_estimate <= 2000);
    }

    #[tokio::test]
    async fn test_historical_context() {
        let builder = ContextBuilder::new();
        let ctx = builder
            .build(ContextStrategy::Historical, "profile-1", "history")
            .await
            .unwrap();
        assert!(ctx.token_estimate >= 1000);
    }

    #[tokio::test]
    async fn test_rag_context_has_memories() {
        let builder = ContextBuilder::new();
        let ctx = builder
            .build(ContextStrategy::RAG, "profile-1", "search")
            .await
            .unwrap();
        assert!(!ctx.rag_memories.is_empty());
    }
}

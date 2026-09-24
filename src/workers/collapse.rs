use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing;

use crate::db::repos::messages::MessagesRepo;
use crate::db::DbPool;
use crate::llm::provider::{ChatMessage, ChatRequest, LLMProvider};
use crate::models::message::estimate_tokens;

/// Background worker that collapses long messages by sending them to an LLM
/// for summarisation.
///
/// Listens on an `mpsc` channel for message IDs whose estimated token count
/// exceeds the collapse threshold. When a message ID is received, the worker
/// fetches the message's full content from the database, sends it to the LLM
/// with the configured collapse prompt, and stores the resulting summary back
/// in the `collapsed_content` column.
pub struct CollapseWorker;

impl CollapseWorker {
    /// Start the collapse worker in a new tokio task.
    ///
    /// The worker loops on `rx` forever (or until the channel closes). For
    /// each received message ID it:
    /// 1. Reads the message from the database.
    /// 2. Builds a [`ChatRequest`] with the collapse prompt + message content.
    /// 3. Calls `llm_provider.chat()`.
    /// 4. Writes the LLM response into `collapsed_content`.
    ///
    /// Returns a [`JoinHandle`] that can be awaited or aborted.
    pub fn start(
        db: DbPool,
        llm_provider: Arc<dyn LLMProvider>,
        mut rx: mpsc::Receiver<String>,
        collapse_prompt: String,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(message_id) = rx.recv().await {
                tracing::info!(message_id = %message_id, "CollapseWorker processing message");

                // 1. Read message from DB
                let msg = {
                    let conn = db.lock().unwrap();
                    MessagesRepo::find_by_id(&conn, &message_id).ok().flatten()
                };

                if let Some(msg) = msg {
                    // 2. Build LLM request
                    let request = ChatRequest {
                        model: "mistralai/mistral-small".to_string(),
                        messages: vec![
                            ChatMessage {
                                role: "system".to_string(),
                                content: collapse_prompt.clone(),
                                tool_calls: None,
                                tool_result: None,
                                tool_call_id: None,
                            },
                            ChatMessage {
                                role: "user".to_string(),
                                content: msg.content.clone(),
                                tool_calls: None,
                                tool_result: None,
                                tool_call_id: None,
                            },
                        ],
                        tools: None,
                        temperature: Some(0.3),
                        max_tokens: Some(1024),
                        stream: false,
                    };

                    // 3. Call LLM
                    match llm_provider.chat(request).await {
                        Ok(response) => {
                            let collapsed = response.message.content;
                            let collapsed_tokens = estimate_tokens(&collapsed);

                            // 4. Update DB
                            let conn = db.lock().unwrap();
                            let result = conn.execute(
                                "UPDATE messages SET collapsed_content = ?1, collapsed_tokens_count = ?2 WHERE id = ?3",
                                rusqlite::params![collapsed, collapsed_tokens, message_id],
                            );
                            if let Err(e) = result {
                                tracing::error!(message_id = %message_id, error = %e, "CollapseWorker failed to update DB");
                            } else {
                                tracing::info!(message_id = %message_id, collapsed_tokens = %collapsed_tokens, "CollapseWorker completed");
                            }
                        }
                        Err(e) => {
                            tracing::error!(message_id = %message_id, error = %e, "CollapseWorker LLM call failed");
                        }
                    }
                } else {
                    tracing::warn!(message_id = %message_id, "CollapseWorker: message not found");
                }
            }
            tracing::info!("CollapseWorker shutting down");
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repos::conversations::ConversationsRepo;
    use crate::db::repos::messages::MessagesRepo;
    use crate::db::schema::run_migrations;
    use crate::llm::provider::{ChatMessage, ChatRequest, ChatResponse, LLMError, TokenUsage};
    use async_trait::async_trait;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    /// A mock LLM provider that records every [`ChatRequest`] it receives and
    /// returns a canned response.
    struct MockLLMProvider {
        pub calls: Arc<Mutex<Vec<ChatRequest>>>,
    }

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
            self.calls.lock().unwrap().push(request);
            Ok(ChatResponse {
                message: ChatMessage {
                    role: "assistant".into(),
                    content: "Resumen del mensaje.".into(),
                    tool_calls: None,
                    tool_result: None,
                    tool_call_id: None,
                },
                usage: Some(TokenUsage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                }),
            })
        }

        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn tokio_stream::Stream<
                            Item = Result<crate::llm::provider::StreamEvent, LLMError>,
                        > + Send,
                >,
            >,
            LLMError,
        > {
            unimplemented!("chat_stream not used in tests")
        }

        async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
            unimplemented!("embed not used in tests")
        }
    }

    /// Helper: create an in-memory DbPool with migrations and a conversation.
    fn test_db_with_conversation() -> (DbPool, String) {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let conv = ConversationsRepo::create(&conn, "Test Conv").unwrap();
        let pool: DbPool = Arc::new(Mutex::new(conn));
        (pool, conv.id)
    }

    /// Helper: create a long message (> 2000 tokens) and return its id.
    fn create_long_message(pool: &DbPool, conv_id: &str) -> String {
        let long_content = "x".repeat(8000);
        let db = pool.lock().unwrap();
        let msg = MessagesRepo::create(&db, conv_id, "user", &long_content, None, None, 2000, None)
            .unwrap();
        msg.id
    }

    /// Given a running CollapseWorker with a mock LLM provider,
    /// when a message ID is sent through the channel,
    /// then the LLM provider's `chat()` must be called with the correct request.
    #[tokio::test]
    async fn test_worker_receives_message_id_and_calls_llm() {
        let (pool, conv_id) = test_db_with_conversation();
        let msg_id = create_long_message(&pool, &conv_id);

        let (tx, rx) = mpsc::channel::<String>(16);
        let calls: Arc<Mutex<Vec<ChatRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let mock = Arc::new(MockLLMProvider {
            calls: calls.clone(),
        });

        let prompt = "Resume el siguiente mensaje en español.".to_string();
        let _handle = CollapseWorker::start(pool.clone(), mock, rx, prompt);

        // Send message ID through channel
        tx.send(msg_id.clone()).await.unwrap();

        // Give the worker time to process
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // The LLM must have been called
        let call_count = calls.lock().unwrap().len();
        assert!(
            call_count > 0,
            "LLM provider was not called – worker did not process the message"
        );

        // The request must contain the collapse prompt as system message
        let request = calls.lock().unwrap()[0].clone();
        assert!(
            request.messages.iter().any(|m| m.role == "system"),
            "Request should contain a system message with the collapse prompt"
        );
    }

    /// Given a running CollapseWorker,
    /// when a message is collapsed,
    /// then the database must have its `collapsed_content` updated.
    #[tokio::test]
    async fn test_worker_updates_collapsed_content_in_db() {
        let (pool, conv_id) = test_db_with_conversation();
        let msg_id = create_long_message(&pool, &conv_id);

        let (tx, rx) = mpsc::channel::<String>(16);
        let calls: Arc<Mutex<Vec<ChatRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let mock = Arc::new(MockLLMProvider { calls });

        let prompt = "Resume el siguiente mensaje.".to_string();
        let _handle = CollapseWorker::start(pool.clone(), mock, rx, prompt);

        // Send message ID
        tx.send(msg_id.clone()).await.unwrap();

        // Give the worker time to process
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify the DB was updated
        let db = pool.lock().unwrap();
        let msg = MessagesRepo::find_by_id(&db, &msg_id)
            .unwrap()
            .expect("Message should exist");

        assert!(
            msg.collapsed_content.is_some(),
            "collapsed_content should be set after worker processes the message"
        );
        assert_eq!(
            msg.collapsed_content.as_deref(),
            Some("Resumen del mensaje."),
            "collapsed_content should match the mock LLM response"
        );
        assert!(
            msg.collapsed_tokens_count > 0,
            "collapsed_tokens_count should be > 0"
        );
    }

    /// Given a custom collapse prompt,
    /// when the worker processes a message,
    /// then the prompt sent to the LLM must match the configured one.
    #[tokio::test]
    async fn test_worker_uses_custom_prompt() {
        let (pool, conv_id) = test_db_with_conversation();
        let msg_id = create_long_message(&pool, &conv_id);

        let (tx, rx) = mpsc::channel::<String>(16);
        let calls: Arc<Mutex<Vec<ChatRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let mock = Arc::new(MockLLMProvider {
            calls: calls.clone(),
        });

        let custom_prompt = "CUSTOM: Summarize this in one sentence.".to_string();
        let _handle = CollapseWorker::start(pool.clone(), mock, rx, custom_prompt.clone());

        // Send message ID
        tx.send(msg_id.clone()).await.unwrap();

        // Give the worker time to process
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify the system message contains our custom prompt
        let request = calls.lock().unwrap()[0].clone();
        let system_msg = request
            .messages
            .iter()
            .find(|m| m.role == "system")
            .expect("Should have a system message");

        assert_eq!(
            system_msg.content, custom_prompt,
            "System message should contain the custom collapse prompt"
        );
    }
}

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::llm::provider::{ChatMessage, ChatRequest, LLMProvider, ToolCall};
use crate::orchestrator::context_builder::ContextBuilder;
use crate::orchestrator::context_classifier::ContextClassifier;
use crate::orchestrator::guardrails::{GuardrailResult, Guardrails};
use crate::tools::registry::ToolRegistry;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_iterations: usize,
    pub max_tokens_per_turn: u32,
    pub enable_reflection: bool,
    pub system_prompt_template: String,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_tokens_per_turn: 4096,
            enable_reflection: true,
            system_prompt_template:
                "Eres Alfred, un asistente de IA con actitud de mayordomo británico. \
Eres sarcástico, irónico y burlón, pero siempre resolutivo. \
Tus respuestas son ingeniosas y con humor seco, pero NUNCA insultantes. \
Mantienes un tono elegante y mordaz, como Jeeves con experiencia en tecnología.\n\n\
Siempre respondes usando Markdown con:\n\
- **negritas** para énfasis fuerte\n\
- *cursivas* para matices o énfasis sutil\n\
- Listas con viñetas cuando enumeras opciones\n\
- Emojis relevantes para amenizar (🌤️ clima, 📍 ubicación, 🍽️ comidas, ✅ hábitos, etc.)\n\
- Formato limpio y legible\n\n\
Ayudas al usuario con clima, comidas, hábitos, búsquedas y gestión personal. \
Usas herramientas cuando es necesario, pero siempre con comentario sarcástico."
                    .into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub message: String,
    pub tool_calls: Vec<ToolCallInfo>,
    pub reflection: Option<Reflection>,
    pub iterations: usize,
}

#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub name: String,
    pub arguments: Value,
    pub result: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct Reflection {
    pub is_coherent: bool,
    pub is_complete: bool,
    pub needs_clarification: Option<String>,
    pub suggested_followup: Option<String>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("LLM error: {0}")]
    LLMError(String),
    #[error("Tool error: {0}")]
    ToolError(String),
    #[error("Guardrail error: {0}")]
    GuardrailError(String),
    #[error("Context error: {0}")]
    ContextError(String),
    #[error("Max iterations exceeded")]
    MaxIterationsExceeded,
    #[error("Internal error: {0}")]
    Internal(String),
}

// ---------------------------------------------------------------------------
// SSE events (used by streaming endpoint)
// ---------------------------------------------------------------------------

/// Events sent to the frontend via SSE during orchestrator streaming.
///
/// Each variant is serialized with a `"type"` tag that the frontend
/// uses to distinguish event kinds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SSEEvent {
    /// A text chunk of the assistant's response.
    #[serde(rename = "chunk")]
    Chunk { content: String },

    /// A tool was invoked by the LLM.
    #[serde(rename = "tool_call")]
    ToolCall { name: String, args: Value },

    /// The result of a tool execution.
    #[serde(rename = "tool_result")]
    ToolResult { name: String, success: bool },

    /// Streaming is complete; the assistant message has been persisted.
    #[serde(rename = "done")]
    Done {
        message_id: String,
        user_message_id: String,
    },

    /// An error occurred during processing.
    #[serde(rename = "error")]
    Error { message: String },

    /// Human-in-the-loop approval is required before a tool can proceed.
    #[serde(rename = "approval_required")]
    ApprovalRequired {
        request_id: String,
        tool_name: String,
        reason: String,
    },

    /// The result of an approval resolution (approved or denied).
    #[serde(rename = "approval_result")]
    ApprovalResult { request_id: String, approved: bool },
}

impl SSEEvent {
    /// Serialise this event to a JSON string suitable for an SSE `data:` field.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"type":"error","message":"Failed to serialize SSE event"}"#.to_string()
        })
    }
}

/// Contextual information from the user's browser (date, time, location).
///
/// Injected as a system message so the LLM can personalise responses
/// (e.g. adjust greetings, interpret relative dates, offer local info).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserContext {
    pub timestamp: String,
    pub timezone: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Orchestrator — ReAct loop
// ---------------------------------------------------------------------------

pub struct Orchestrator {
    pub llm: Arc<dyn LLMProvider>,
    pub registry: Arc<ToolRegistry>,
    pub guardrails: Arc<Guardrails>,
    pub context_builder: Arc<ContextBuilder>,
    pub classifier: ContextClassifier,
    pub config: OrchestratorConfig,
    pub db: Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        registry: Arc<ToolRegistry>,
        guardrails: Arc<Guardrails>,
        context_builder: Arc<ContextBuilder>,
        config: OrchestratorConfig,
        db: Arc<std::sync::Mutex<rusqlite::Connection>>,
    ) -> Self {
        Self {
            llm,
            registry,
            guardrails,
            context_builder,
            classifier: ContextClassifier::new(),
            config,
            db,
        }
    }

    /// Non-streaming entry point: runs the full ReAct loop and returns the
    /// final response together with any tool calls and reflection metadata.
    pub async fn process_message(
        &self,
        conversation_id: &str,
        profile_id: &str,
        user_message: &str,
    ) -> Result<AgentResponse, AgentError> {
        let mut iterations = 0usize;
        let mut all_tool_calls: Vec<ToolCallInfo> = Vec::new();
        let mut messages: Vec<ChatMessage> = Vec::new();

        // 1. Classify intent
        let classification = self.classifier.classify(user_message);

        // 2. Build context (system prompt, optional RAG memories, etc.)
        let ctx = self
            .context_builder
            .build(classification.strategy.clone(), profile_id, user_message)
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;

        // Read settings from DB (max_window_tokens, system_prompt)
        let max_window_tokens = {
            let db_guard = self
                .db
                .lock()
                .map_err(|e| AgentError::Internal(e.to_string()))?;
            crate::db::repos::settings::SettingsRepo::get(&db_guard, "max_window_tokens")
                .ok()
                .flatten()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(10000)
        };

        let custom_prompt = {
            let db_guard = self
                .db
                .lock()
                .map_err(|e| AgentError::Internal(e.to_string()))?;
            crate::db::repos::settings::SettingsRepo::get(&db_guard, "system_prompt")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        };

        // Use custom prompt if set, otherwise use template
        let system_prompt =
            custom_prompt.unwrap_or_else(|| self.config.system_prompt_template.clone());

        // Inject system prompt
        messages.push(ChatMessage {
            role: "system".into(),
            content: system_prompt,
            tool_calls: None,
            tool_result: None,
            tool_call_id: None,
        });

        // Inject RAG memories as context if available
        for memory in &ctx.rag_memories {
            messages.push(ChatMessage {
                role: "system".into(),
                content: format!("[Memory context] {}", memory),
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            });
        }

        // Inject session summary if available
        if let Some(ref summary) = ctx.session_summary {
            messages.push(ChatMessage {
                role: "system".into(),
                content: format!("[Session summary] {}", summary),
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            });
        }

        // Load conversation history from DB using token budget
        {
            let db_guard = self
                .db
                .lock()
                .map_err(|e| AgentError::Internal(e.to_string()))?;
            let history = crate::db::repos::messages::MessagesRepo::list_by_token_budget(
                &db_guard,
                conversation_id,
                max_window_tokens,
            )
            .map_err(|e| AgentError::Internal(e.to_string()))?;
            for msg in &history {
                let tool_calls: Option<Vec<ToolCall>> = msg
                    .tool_calls
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                messages.push(ChatMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    tool_calls,
                    tool_result: msg.tool_results.clone(),
                    tool_call_id: None,
                });
            }
        }

        // 3. Add the user message
        messages.push(ChatMessage {
            role: "user".into(),
            content: user_message.to_string(),
            tool_calls: None,
            tool_result: None,
            tool_call_id: None,
        });

        // 4. ReAct loop
        loop {
            if iterations >= self.config.max_iterations {
                return Err(AgentError::MaxIterationsExceeded);
            }

            let request = ChatRequest {
                model: "default".into(),
                messages: messages.clone(),
                tools: Some(self.registry.definitions()),
                temperature: None,
                max_tokens: Some(self.config.max_tokens_per_turn),
                stream: false,
            };

            let response = self
                .llm
                .chat(request)
                .await
                .map_err(|e| AgentError::LLMError(e.to_string()))?;

            iterations += 1;

            // Check for tool calls in the LLM response
            let has_tool_calls = response
                .message
                .tool_calls
                .as_ref()
                .map(|calls| !calls.is_empty())
                .unwrap_or(false);

            if has_tool_calls {
                let tool_calls = response.message.tool_calls.clone().unwrap();

                // Push assistant message with tool calls
                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: response.message.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_result: None,
                    tool_call_id: None,
                });

                // Execute each tool call
                for tc in &tool_calls {
                    // Guardrails check
                    let guardrail = self
                        .guardrails
                        .check(&tc.name, &tc.arguments)
                        .map_err(|e| AgentError::GuardrailError(e.to_string()))?;

                    match guardrail {
                        GuardrailResult::Allowed { .. } => {
                            // Execute the tool
                            let tool_result = self
                                .registry
                                .execute(&tc.name, tc.arguments.clone())
                                .await
                                .map_err(|e| AgentError::ToolError(e.to_string()))?;

                            let result_value = tool_result.data;

                            // Track for the final response
                            all_tool_calls.push(ToolCallInfo {
                                name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                                result: Some(result_value.clone()),
                            });

                            // Push tool result message for the LLM
                            messages.push(ChatMessage {
                                role: "tool".into(),
                                content: serde_json::to_string(&result_value).unwrap_or_default(),
                                tool_calls: None,
                                tool_result: Some(result_value),
                                tool_call_id: Some(tc.id.clone()),
                            });
                        }
                        GuardrailResult::RequiresApproval { request_id } => {
                            return Err(AgentError::GuardrailError(format!(
                                "Tool '{}' requires explicit approval (request_id: {})",
                                tc.name, request_id
                            )));
                        }
                    }
                }

                // Continue the loop so the LLM can produce the final answer
                // (or call more tools).
                continue;
            }

            // No tool calls → this is the final answer
            let message = response.message.content;

            // Optional reflection
            let reflection = if self.config.enable_reflection {
                let analyzer = ReflectionAnalyzer::new(self.llm.clone());
                analyzer.analyze(&messages, &message).await.ok()
            } else {
                None
            };

            return Ok(AgentResponse {
                message,
                tool_calls: all_tool_calls,
                reflection,
                iterations,
            });
        }
    }

    /// Streaming entry point: runs the ReAct loop and emits [`SSEEvent`] values
    /// over the provided channel so the frontend can receive them incrementally.
    pub async fn process_message_stream(
        &self,
        conversation_id: &str,
        profile_id: &str,
        user_message: &str,
        browser_context: Option<BrowserContext>,
        tx: mpsc::Sender<SSEEvent>,
    ) -> Result<(), AgentError> {
        tracing::info!(
            conversation_id,
            user_message_len = %user_message.len(),
            "🚀 Orchestrator processing message stream"
        );

        let mut iterations = 0usize;
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut used_tools: Vec<String> = Vec::new();

        // 1. Classify intent
        let classification = self.classifier.classify(user_message);

        // 2. Build context
        let ctx = self
            .context_builder
            .build(classification.strategy.clone(), profile_id, user_message)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "❌ Orchestrator error");
                AgentError::ContextError(e.to_string())
            })?;

        // Read settings from DB (max_window_tokens, system_prompt)
        let max_window_tokens = {
            let db_guard = self
                .db
                .lock()
                .map_err(|e| AgentError::Internal(e.to_string()))?;
            crate::db::repos::settings::SettingsRepo::get(&db_guard, "max_window_tokens")
                .ok()
                .flatten()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(10000)
        };

        let custom_prompt = {
            let db_guard = self
                .db
                .lock()
                .map_err(|e| AgentError::Internal(e.to_string()))?;
            crate::db::repos::settings::SettingsRepo::get(&db_guard, "system_prompt")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
        };

        // Use custom prompt if set, otherwise use template
        let system_prompt =
            custom_prompt.unwrap_or_else(|| self.config.system_prompt_template.clone());

        // 3. ReAct loop
        tracing::debug!(
            system_prompt_len = %ctx.system_prompt.len(),
            rag_count = %ctx.rag_memories.len(),
            "Context built for ReAct loop"
        );

        messages.push(ChatMessage {
            role: "system".into(),
            content: system_prompt,
            tool_calls: None,
            tool_result: None,
            tool_call_id: None,
        });

        for memory in &ctx.rag_memories {
            messages.push(ChatMessage {
                role: "system".into(),
                content: format!("[Memory context] {}", memory),
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            });
        }

        if let Some(ref summary) = ctx.session_summary {
            messages.push(ChatMessage {
                role: "system".into(),
                content: format!("[Session summary] {}", summary),
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            });
        }

        // Inject browser context (date/time/location from user's browser)
        if let Some(ref ctx) = browser_context {
            let mut parts = vec![format!(
                "[Context] Hoy es {}. Zona horaria: {}.",
                ctx.timestamp, ctx.timezone
            )];
            if let (Some(lat), Some(lon)) = (ctx.latitude, ctx.longitude) {
                parts.push(format!("Coordenadas: ({:.4}, {:.4}).", lat, lon));
            }
            if let Some(ref name) = ctx.location_name {
                parts.push(format!("Ubicación: {}.", name));
            }
            messages.push(ChatMessage {
                role: "system".into(),
                content: parts.join(" "),
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            });
        }

        // Load conversation history from DB using token budget
        {
            let db_guard = self
                .db
                .lock()
                .map_err(|e| AgentError::Internal(e.to_string()))?;
            let history = crate::db::repos::messages::MessagesRepo::list_by_token_budget(
                &db_guard,
                conversation_id,
                max_window_tokens,
            )
            .map_err(|e| AgentError::Internal(e.to_string()))?;
            for msg in &history {
                let tool_calls: Option<Vec<ToolCall>> = msg
                    .tool_calls
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                messages.push(ChatMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    tool_calls,
                    tool_result: msg.tool_results.clone(),
                    tool_call_id: None,
                });
            }
        }

        messages.push(ChatMessage {
            role: "user".into(),
            content: user_message.to_string(),
            tool_calls: None,
            tool_result: None,
            tool_call_id: None,
        });

        // Persist user message to DB
        let user_message_id = {
            let db_guard = self
                .db
                .lock()
                .map_err(|e| AgentError::Internal(e.to_string()))?;
            let msg = crate::db::repos::messages::MessagesRepo::create(
                &db_guard,
                conversation_id,
                "user",
                user_message,
                None,
                None,
                2000,
                None,
            )
            .map_err(|e| AgentError::Internal(e.to_string()))?;
            msg.id
        };

        loop {
            if iterations >= self.config.max_iterations {
                tracing::error!(error = %AgentError::MaxIterationsExceeded, "❌ Orchestrator error");
                let _ = tx
                    .send(SSEEvent::Error {
                        message: "Max iterations exceeded".into(),
                    })
                    .await;
                return Err(AgentError::MaxIterationsExceeded);
            }

            tracing::debug!(iteration = %iterations, "ReAct loop iteration");

            let request = ChatRequest {
                model: "default".into(),
                messages: messages.clone(),
                tools: Some(self.registry.definitions()),
                temperature: None,
                max_tokens: Some(self.config.max_tokens_per_turn),
                stream: false,
            };

            let response = self.llm.chat(request).await.map_err(|e| {
                tracing::error!(error = %e, "❌ Orchestrator error");
                AgentError::LLMError(e.to_string())
            })?;

            iterations += 1;

            let has_tool_calls = response
                .message
                .tool_calls
                .as_ref()
                .map(|calls| !calls.is_empty())
                .unwrap_or(false);

            tracing::debug!(
                iteration = %iterations,
                has_tool_calls,
                content_preview = %response.message.content.chars().take(50).collect::<String>(),
                "LLM response received"
            );

            if has_tool_calls {
                let tool_calls = response.message.tool_calls.clone().unwrap();

                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: response.message.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_result: None,
                    tool_call_id: None,
                });

                for tc in &tool_calls {
                    tracing::info!(tool_name = %tc.name, "🔧 Executing tool call");

                    // Emit tool_call event
                    if tx
                        .send(SSEEvent::ToolCall {
                            name: tc.name.clone(),
                            args: tc.arguments.clone(),
                        })
                        .await
                        .is_err()
                    {
                        return Ok(()); // client disconnected
                    }

                    // Guardrails check
                    let guardrail =
                        self.guardrails
                            .check(&tc.name, &tc.arguments)
                            .map_err(|e| {
                                tracing::error!(error = %e, "❌ Orchestrator error");
                                AgentError::GuardrailError(e.to_string())
                            })?;

                    match guardrail {
                        GuardrailResult::Allowed { .. } => {
                            let tool_result =
                                self.registry.execute(&tc.name, tc.arguments.clone()).await;

                            match tool_result {
                                Ok(result) => {
                                    // Track the tool name for the footer
                                    used_tools.push(tc.name.clone());

                                    // Emit success event
                                    let _ = tx
                                        .send(SSEEvent::ToolResult {
                                            name: tc.name.clone(),
                                            success: true,
                                        })
                                        .await;

                                    messages.push(ChatMessage {
                                        role: "tool".into(),
                                        content: serde_json::to_string(&result.data)
                                            .unwrap_or_default(),
                                        tool_calls: None,
                                        tool_result: Some(result.data),
                                        tool_call_id: Some(tc.id.clone()),
                                    });
                                }
                                Err(e) => {
                                    let _ = tx
                                        .send(SSEEvent::ToolResult {
                                            name: tc.name.clone(),
                                            success: false,
                                        })
                                        .await;

                                    messages.push(ChatMessage {
                                        role: "tool".into(),
                                        content: format!("Error: {}", e),
                                        tool_calls: None,
                                        tool_result: None,
                                        tool_call_id: Some(tc.id.clone()),
                                    });
                                }
                            }
                        }
                        GuardrailResult::RequiresApproval { request_id } => {
                            let _ = tx
                                .send(SSEEvent::ApprovalRequired {
                                    request_id: request_id.clone(),
                                    tool_name: tc.name.clone(),
                                    reason: format!(
                                        "Tool '{}' requires explicit approval",
                                        tc.name
                                    ),
                                })
                                .await;

                            let err = AgentError::GuardrailError(format!(
                                "Tool '{}' requires explicit approval (request_id: {})",
                                tc.name, request_id
                            ));
                            tracing::error!(error = %err, "❌ Orchestrator error");
                            return Err(err);
                        }
                    }
                }

                continue;
            }

            // Final answer — send chunks
            let final_text = if !used_tools.is_empty() {
                let footer = format!("\n\n---\n🔧 {}", used_tools.join(" · "));
                format!("{}{}", response.message.content, footer)
            } else {
                response.message.content.clone()
            };
            for chunk in final_text
                .chars()
                .collect::<Vec<_>>()
                .chunks(10)
                .map(|c| c.iter().collect::<String>())
            {
                if tx.send(SSEEvent::Chunk { content: chunk }).await.is_err() {
                    return Ok(()); // client disconnected
                }
            }

            // Persist assistant message to DB (capture the real UUID)
            let assistant_message_id = {
                let db_guard = self
                    .db
                    .lock()
                    .map_err(|e| AgentError::Internal(e.to_string()))?;
                let msg = crate::db::repos::messages::MessagesRepo::create(
                    &db_guard,
                    conversation_id,
                    "assistant",
                    &final_text,
                    None,
                    None,
                    2000,
                    None,
                )
                .map_err(|e| AgentError::Internal(e.to_string()))?;
                msg.id
            };

            let _ = tx
                .send(SSEEvent::Done {
                    message_id: assistant_message_id,
                    user_message_id: user_message_id.clone(),
                })
                .await;

            tracing::info!(iterations, "✅ Orchestrator finished processing message");

            return Ok(());
        }
    }
}

// ---------------------------------------------------------------------------
// Reflection analyzer
// ---------------------------------------------------------------------------

pub struct ReflectionAnalyzer {
    llm: Arc<dyn LLMProvider>,
}

impl ReflectionAnalyzer {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self { llm }
    }

    /// Ask the LLM to reflect on its own response — checking coherence,
    /// completeness, and whether clarification is needed.
    pub async fn analyze(
        &self,
        conversation: &[ChatMessage],
        response: &str,
    ) -> Result<Reflection, AgentError> {
        let conv_json = serde_json::to_string(conversation).unwrap_or_default();

        let prompt = format!(
            r#"Analyze this assistant response:

Conversation:
{}

Response:
{}

Answer the following yes/no questions (reply with one JSON object):
- Is the response coherent?
- Is the response complete?
- Does it need clarification from the user?
- What suggested follow-up would you propose?

Respond in JSON format:
{{"is_coherent": bool, "is_complete": bool, "needs_clarification": bool, "followup": "..."}}"#,
            conv_json, response
        );

        let request = ChatRequest {
            model: "default".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: prompt,
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            }],
            tools: None,
            temperature: Some(0.3),
            max_tokens: Some(256),
            stream: false,
        };

        match self.llm.chat(request).await {
            Ok(resp) => {
                // Try to parse JSON from the response
                let content = resp.message.content.trim().to_lowercase();

                // Fallback heuristic if JSON parsing fails
                let is_coherent =
                    !content.contains("not coherent") && !content.contains("incoherent");
                let is_complete =
                    !content.contains("not complete") && !content.contains("incomplete");

                // Try structured JSON parsing
                let parsed = serde_json::from_str::<serde_json::Value>(resp.message.content.trim());

                if let Ok(json) = parsed {
                    let coherent = json
                        .get("is_coherent")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(is_coherent);
                    let complete = json
                        .get("is_complete")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(is_complete);
                    let needs_clar = json
                        .get("needs_clarification")
                        .and_then(|v| v.as_bool())
                        .unwrap_or_else(|| {
                            content.contains("clarification") || content.contains("unclear")
                        });
                    let followup = json
                        .get("followup")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    return Ok(Reflection {
                        is_coherent: coherent,
                        is_complete: complete,
                        needs_clarification: if needs_clar {
                            Some("I may need more details. Could you clarify?".into())
                        } else {
                            None
                        },
                        suggested_followup: followup,
                    });
                }

                // Heuristic fallback
                Ok(Reflection {
                    is_coherent,
                    is_complete,
                    needs_clarification: if content.contains("clarification")
                        || content.contains("unclear")
                    {
                        Some("I may need more details. Could you clarify?".into())
                    } else {
                        None
                    },
                    suggested_followup: if !is_complete {
                        Some("Is there anything else you'd like to know?".into())
                    } else {
                        None
                    },
                })
            }
            Err(_) => Ok(Reflection {
                is_coherent: true,
                is_complete: true,
                needs_clarification: None,
                suggested_followup: Some("¿Necesitas algo más?".into()),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::{ChatResponse, LLMError, StreamEvent};
    use crate::tools::permission::Permission;
    use crate::tools::r#trait::{Tool, ToolError, ToolResult};
    use std::pin::Pin;
    use std::sync::Mutex;
    use tokio_stream::Stream;

    // --- SSE event serialization tests (legacy, must keep passing) -----------

    #[test]
    fn test_chunk_serialization() {
        let event = SSEEvent::Chunk {
            content: "Hello".to_string(),
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"chunk""#));
        assert!(json.contains(r#""content":"Hello""#));
    }

    #[test]
    fn test_done_serialization() {
        let event = SSEEvent::Done {
            message_id: "msg-123".to_string(),
            user_message_id: "msg-user-123".to_string(),
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"done""#));
        assert!(json.contains(r#""message_id":"msg-123""#));
        assert!(json.contains(r#""user_message_id":"msg-user-123""#));
    }

    #[test]
    fn test_done_serialization_with_user_message_id() {
        let event = SSEEvent::Done {
            message_id: "msg-1".to_string(),
            user_message_id: "user-1".to_string(),
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"done""#));
        assert!(json.contains(r#""message_id":"msg-1""#));
        assert!(json.contains(r#""user_message_id":"user-1""#));
    }

    #[test]
    fn test_approval_required_serialization() {
        let event = SSEEvent::ApprovalRequired {
            request_id: "req-1".to_string(),
            tool_name: "delete_file".to_string(),
            reason: "Requires explicit approval".to_string(),
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"approval_required""#));
        assert!(json.contains(r#""tool_name":"delete_file""#));
    }

    #[test]
    fn test_error_serialization() {
        let event = SSEEvent::Error {
            message: "Something went wrong".to_string(),
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains(r#""message":"Something went wrong""#));
    }

    #[test]
    fn test_tool_call_serialization() {
        let event = SSEEvent::ToolCall {
            name: "get_weather".to_string(),
            args: serde_json::json!({"city": "Madrid"}),
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"tool_call""#));
        assert!(json.contains(r#""name":"get_weather""#));
        assert!(json.contains(r#""city":"Madrid""#));
    }

    #[test]
    fn test_tool_result_serialization() {
        let event = SSEEvent::ToolResult {
            name: "get_weather".to_string(),
            success: true,
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"tool_result""#));
        assert!(json.contains(r#""name":"get_weather""#));
        assert!(json.contains(r#""success":true"#));
    }

    #[test]
    fn test_approval_result_serialization() {
        let event = SSEEvent::ApprovalResult {
            request_id: "req-1".to_string(),
            approved: true,
        };
        let json = event.to_json_string();
        assert!(json.contains(r#""type":"approval_result""#));
        assert!(json.contains(r#""request_id":"req-1""#));
        assert!(json.contains(r#""approved":true"#));
    }

    // --- New Orchestrator / AgentError / config tests ------------------------

    #[test]
    fn test_orchestrator_config_defaults() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.max_tokens_per_turn, 4096);
        assert!(config.enable_reflection);
        assert!(config.system_prompt_template.contains("Alfred"));
    }

    #[test]
    fn test_agent_error_display_llm() {
        let err = AgentError::LLMError("rate limited".into());
        assert!(err.to_string().contains("rate limited"));
    }

    #[test]
    fn test_agent_error_display_tool() {
        let err = AgentError::ToolError("not found".into());
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_agent_error_display_guardrail() {
        let err = AgentError::GuardrailError("denied".into());
        assert!(err.to_string().contains("denied"));
    }

    #[test]
    fn test_agent_error_display_context() {
        let err = AgentError::ContextError("profile missing".into());
        assert!(err.to_string().contains("profile missing"));
    }

    #[test]
    fn test_agent_error_display_max_iterations() {
        let err = AgentError::MaxIterationsExceeded;
        assert_eq!(err.to_string(), "Max iterations exceeded");
    }

    #[test]
    fn test_agent_error_display_internal() {
        let err = AgentError::Internal("something broke".into());
        assert!(err.to_string().contains("something broke"));
    }

    #[test]
    fn test_agent_error_impl_debug_and_clone() {
        let err = AgentError::LLMError("oops".into());
        let cloned = err.clone();
        assert!(format!("{:?}", cloned).contains("oops"));
    }

    #[test]
    fn test_tool_call_info_construction() {
        let info = ToolCallInfo {
            name: "search".into(),
            arguments: serde_json::json!({"q": "test"}),
            result: Some(serde_json::json!({"results": []})),
        };
        assert_eq!(info.name, "search");
        assert!(info.result.is_some());
    }

    #[test]
    fn test_reflection_defaults() {
        let r = Reflection {
            is_coherent: true,
            is_complete: false,
            needs_clarification: Some("Please clarify".into()),
            suggested_followup: None,
        };
        assert!(r.is_coherent);
        assert!(!r.is_complete);
        assert!(r.needs_clarification.is_some());
        assert!(r.suggested_followup.is_none());
    }

    // -----------------------------------------------------------------------
    // Mock LLM that returns a tool call on first invocation, then plain text
    // -----------------------------------------------------------------------

    struct MockLLMWithToolThenAnswer {
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl LLMProvider for MockLLMWithToolThenAnswer {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            if *count == 1 {
                // First call: return a tool call that the orchestrator will execute
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Let me check the weather.".into(),
                        tool_calls: Some(vec![ToolCall {
                            id: "call-1".into(),
                            name: "weather".into(),
                            arguments: serde_json::json!({"latitude": 40.4168, "longitude": -3.7038}),
                        }]),
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            } else {
                // Second call: return final answer without tool calls
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "The weather in Madrid is 22°C and sunny.".into(),
                        tool_calls: None,
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            }
        }

        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            panic!("chat_stream not used in mock")
        }

        async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
            Ok(vec![])
        }
    }

    /// Dummy tool that always succeeds — used to exercise the tool tracking code.
    struct MockWeatherTool;

    #[async_trait::async_trait]
    impl Tool for MockWeatherTool {
        fn name(&self) -> &'static str {
            "weather"
        }

        fn description(&self) -> &'static str {
            "Get weather forecast"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        fn permission(&self) -> Permission {
            Permission::NoConfirm
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult {
                success: true,
                data: serde_json::json!({"temperature": 22.0, "description": "sunny"}),
                message: None,
            })
        }
    }

    // -----------------------------------------------------------------------
    // Streaming integration test — verifies tool footer is added
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_process_message_stream_adds_tool_footer() {
        use crate::db::repos::conversations::ConversationsRepo;
        use crate::db::repos::messages::MessagesRepo;
        use crate::db::schema::run_migrations;

        // 1. Create in-memory SQLite connection and run migrations
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let db = Arc::new(std::sync::Mutex::new(conn));

        // 2. Create a conversation in the DB
        let conv = ConversationsRepo::create(&db.lock().unwrap(), "Test Footer").unwrap();

        // 3. Create a registry with a mock weather tool
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockWeatherTool));
        let registry = Arc::new(registry);

        // 4. Create mock LLM that returns tool call then answer
        let llm = Arc::new(MockLLMWithToolThenAnswer {
            call_count: Arc::new(Mutex::new(0)),
        });
        let guardrails = Arc::new(Guardrails::new(registry.clone()));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            db.clone(),
        );

        // 5. Call process_message_stream
        let (tx, _rx) = mpsc::channel(100);
        let result = orchestrator
            .process_message_stream(&conv.id, "profile-id", "What's the weather?", None, tx)
            .await;

        assert!(result.is_ok(), "Stream should succeed");

        // 6. Query the DB for the assistant message and verify footer
        let db_guard = db.lock().unwrap();
        let (messages, _) =
            MessagesRepo::list_by_conversation(&db_guard, &conv.id, 100, None).unwrap();
        let assistant_messages: Vec<_> =
            messages.iter().filter(|m| m.role == "assistant").collect();

        assert!(
            !assistant_messages.is_empty(),
            "Expected at least one assistant message in DB"
        );

        let last_msg = assistant_messages.last().unwrap();
        assert!(
            last_msg.content.contains("🔧 weather"),
            "Assistant message should contain tool footer with '🔧 weather', got: {}",
            last_msg.content
        );
        assert!(
            last_msg.content.contains("---"),
            "Assistant message should contain '---' separator in footer, got: {}",
            last_msg.content
        );
        assert!(
            last_msg
                .content
                .contains("The weather in Madrid is 22°C and sunny."),
            "Assistant message should contain the original content, got: {}",
            last_msg.content
        );
    }
}

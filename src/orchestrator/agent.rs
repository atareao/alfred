use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

use crate::models::stats::LastApiCall;
use tokio::sync::mpsc;

use crate::db::repos::stats::StatsRepo;
use crate::llm::provider::{ChatMessage, ChatRequest, LLMProvider, StreamEvent, ToolCall};
use crate::orchestrator::context_builder::ContextBuilder;
use crate::orchestrator::context_classifier::ContextClassifier;
use crate::orchestrator::guardrails::{GuardrailResult, Guardrails};
use crate::tools::geo_utils::reverse_geocode;
use crate::tools::r#trait::ToolResult;
use crate::tools::registry::ToolRegistry;
use crate::tools::time_format::format_browser_timestamp;
use futures::StreamExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_iterations: usize,
    pub max_tokens_per_turn: u32,
    pub model: String,
    pub enable_reflection: bool,
    /// Token threshold above which a message is sent to the collapse worker.
    pub collapse_threshold_tokens: usize,
}

/// Minimal generic system prompt used only when `settings.system_prompt` is
/// missing or empty. The real personality prompt lives in the database
/// (seeded by migration `20260929000001_prompts.sql`).
const DEFAULT_SYSTEM_PROMPT_FALLBACK: &str = "You are Valet, a helpful AI assistant.";

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_tokens_per_turn: 4096,
            model: "default".into(),
            enable_reflection: true,
            collapse_threshold_tokens: 2000,
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

impl From<crate::orchestrator::context_builder::ContextError> for AgentError {
    fn from(err: crate::orchestrator::context_builder::ContextError) -> Self {
        AgentError::ContextError(err.to_string())
    }
}

impl From<sqlx::Error> for AgentError {
    fn from(err: sqlx::Error) -> Self {
        AgentError::Internal(err.to_string())
    }
}

impl From<crate::llm::provider::LLMError> for AgentError {
    fn from(err: crate::llm::provider::LLMError) -> Self {
        AgentError::LLMError(err.to_string())
    }
}

impl From<crate::orchestrator::guardrails::GuardrailError> for AgentError {
    fn from(err: crate::orchestrator::guardrails::GuardrailError) -> Self {
        AgentError::GuardrailError(err.to_string())
    }
}

impl From<crate::tools::r#trait::ToolError> for AgentError {
    fn from(err: crate::tools::r#trait::ToolError) -> Self {
        AgentError::ToolError(err.to_string())
    }
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
        location: Option<String>,
        tools_used: Option<String>,
        user_location: Option<String>,
        user_created_at: String,
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

/// Maximum number of times the same tool+operation can be called in one ReAct loop.
const MAX_TOOL_RETRIES: usize = 5;

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
    pub db: SqlitePool,
    pub collapse_tx: Option<mpsc::Sender<String>>,
    pub memory_tx: Option<mpsc::Sender<()>>,
    pub last_api_call: Arc<RwLock<Option<LastApiCall>>>,
}

impl Orchestrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        registry: Arc<ToolRegistry>,
        guardrails: Arc<Guardrails>,
        context_builder: Arc<ContextBuilder>,
        config: OrchestratorConfig,
        db: SqlitePool,
        collapse_tx: Option<mpsc::Sender<String>>,
        memory_tx: Option<mpsc::Sender<()>>,
        last_api_call: Arc<RwLock<Option<LastApiCall>>>,
    ) -> Self {
        Self {
            llm,
            registry,
            guardrails,
            context_builder,
            classifier: ContextClassifier::new(),
            config,
            db,
            collapse_tx,
            memory_tx,
            last_api_call,
        }
    }

    /// Save the last API call data in memory so it can be served by the stats endpoint.
    #[allow(clippy::too_many_arguments)]
    fn save_last_call(
        &self,
        model: &str,
        request_body: Option<&str>,
        response_body: Option<&str>,
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
        cached_tokens: u32,
        reasoning_tokens: u32,
        cost: f64,
        duration_ms: Option<i64>,
        status: &str,
        error_message: Option<&str>,
        tool_calls: Option<&str>,
    ) {
        let last = LastApiCall {
            model: model.to_string(),
            request_body: request_body.map(|s| s.to_string()),
            response_body: response_body.map(|s| s.to_string()),
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cached_tokens,
            reasoning_tokens,
            cost,
            duration_ms,
            status: status.to_string(),
            error_message: error_message.map(|s| s.to_string()),
            tool_calls: tool_calls.map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        *self.last_api_call.write().unwrap() = Some(last);
    }

    /// Non-streaming entry point: runs the full ReAct loop and returns the
    /// final response together with any tool calls and reflection metadata.
    pub async fn process_message(
        &self,
        profile_id: &str,
        user_message: &str,
    ) -> Result<AgentResponse, AgentError> {
        let mut iterations = 0usize;
        let mut all_tool_calls: Vec<ToolCallInfo> = Vec::new();
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut tool_call_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // 1. Classify intent
        let classification = self.classifier.classify(user_message);

        // 2. Build context (system prompt, optional RAG memories, etc.)
        let ctx = self
            .context_builder
            .build(classification.strategy.clone(), profile_id, user_message)
            .await?;

        // Read settings from DB (max_window_tokens, system_prompt)
        let max_window_tokens =
            crate::db::repos::settings::SettingsRepo::get(&self.db, "max_window_tokens")
                .await?
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(10000);

        // The system prompt is stored in the `settings` table (seeded by
        // migration). If it is missing or empty, fall back to a minimal prompt.
        let system_prompt =
            match crate::db::repos::settings::SettingsRepo::get(&self.db, "system_prompt").await? {
                Some(p) if !p.trim().is_empty() => p,
                _ => {
                    tracing::warn!(
                        "settings.system_prompt is missing or empty; using minimal fallback"
                    );
                    DEFAULT_SYSTEM_PROMPT_FALLBACK.to_string()
                }
            };

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
            let history = crate::db::repos::messages::MessagesRepo::list_by_token_budget(
                &self.db,
                max_window_tokens,
            )
            .await?;
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
                model: self.config.model.clone(),
                messages: messages.clone(),
                tools: Some(self.registry.definitions()),
                temperature: None,
                max_tokens: Some(self.config.max_tokens_per_turn),
                stream: false,
            };

            let request_body_str = serde_json::to_string(&request).unwrap_or_default();
            let start = std::time::Instant::now();
            let response = match self.llm.chat(request).await {
                Ok(r) => r,
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as i64;
                    let _ = StatsRepo::record_request(
                        &self.db,
                        &Uuid::new_v4().to_string(),
                        &self.config.model,
                        Some(profile_id),
                        0,
                        0,
                        0,
                        0,
                        0,
                        0.0,
                        Some(duration_ms),
                        "error",
                        Some(&e.to_string()),
                        None,
                        None,
                    )
                    .await;
                    self.save_last_call(
                        &self.config.model,
                        None,
                        None,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0.0,
                        Some(duration_ms),
                        "error",
                        Some(&e.to_string()),
                        None,
                    );
                    return Err(e.into());
                }
            };
            let duration_ms = start.elapsed().as_millis() as i64;

            let prompt_tokens = response
                .usage
                .as_ref()
                .map(|u| u.prompt_tokens as i64)
                .unwrap_or(0);
            let completion_tokens = response
                .usage
                .as_ref()
                .map(|u| u.completion_tokens as i64)
                .unwrap_or(0);
            let total_tokens = prompt_tokens + completion_tokens;

            let _ = StatsRepo::record_request(
                &self.db,
                &Uuid::new_v4().to_string(),
                &self.config.model,
                Some(profile_id),
                prompt_tokens,
                completion_tokens,
                total_tokens,
                response
                    .usage
                    .as_ref()
                    .map(|u| u.cached_tokens as i64)
                    .unwrap_or(0),
                response
                    .usage
                    .as_ref()
                    .map(|u| u.reasoning_tokens as i64)
                    .unwrap_or(0),
                response.usage.as_ref().map(|u| u.cost).unwrap_or(0.0),
                Some(duration_ms),
                "success",
                None,
                None,
                None,
            )
            .await;
            self.save_last_call(
                &self.config.model,
                Some(&request_body_str),
                Some(&serde_json::to_string(&response).unwrap_or_default()),
                prompt_tokens as u32,
                completion_tokens as u32,
                total_tokens as u32,
                response
                    .usage
                    .as_ref()
                    .map(|u| u.cached_tokens)
                    .unwrap_or(0),
                response
                    .usage
                    .as_ref()
                    .map(|u| u.reasoning_tokens)
                    .unwrap_or(0),
                response.usage.as_ref().map(|u| u.cost).unwrap_or(0.0),
                Some(duration_ms),
                "success",
                None,
                None,
            );

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
                            // Check per-tool retry limit (max 3 calls per tool per ReAct loop)
                            let op = tc.arguments.get("operation").and_then(|v| v.as_str());
                            let op_key = match op {
                                Some(op_val) => format!("{}::{}", tc.name, op_val),
                                None => tc.name.clone(),
                            };
                            let tool_count = tool_call_counts.entry(op_key).or_insert(0);
                            *tool_count += 1;
                            if *tool_count > MAX_TOOL_RETRIES {
                                let display_name = match op {
                                    Some(op_val) => format!("{}::{}", tc.name, op_val),
                                    None => tc.name.clone(),
                                };
                                let msg = format!(
                                    "Tool '{}' has been called 3 times. No more retries allowed. Inform the user and suggest alternatives.",
                                    display_name
                                );
                                messages.push(ChatMessage {
                                    role: "tool".into(),
                                    content: msg,
                                    tool_calls: None,
                                    tool_result: None,
                                    tool_call_id: Some(tc.id.clone()),
                                });
                                continue;
                            }

                            // Inject profile_id from authenticated session
                            let mut args = tc.arguments.clone();
                            if let Some(obj) = args.as_object_mut() {
                                obj.insert("profile_id".into(), serde_json::json!(profile_id));
                            }

                            // Execute the tool
                            let tool_result = self.registry.execute(&tc.name, args).await?;

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
            let reflection: Option<Reflection> = if self.config.enable_reflection {
                let analyzer =
                    ReflectionAnalyzer::new(self.llm.clone(), self.last_api_call.clone());
                Some(
                    analyzer
                        .analyze(&messages, &message, &self.db, profile_id)
                        .await?,
                )
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

    /// Resolve the user's current location name from settings or reverse geocoding.
    /// Caches the result back to settings so subsequent calls are instant.
    async fn resolve_location(&self) -> Option<String> {
        // First try stored location_name
        if let Some(name) = crate::db::repos::settings::SettingsRepo::get(&self.db, "location_name")
            .await
            .ok()
            .flatten()
        {
            return Some(name);
        }

        // Fall back to reverse geocoding from stored lat/lon
        let lat = crate::db::repos::settings::SettingsRepo::get(&self.db, "latitude")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<f64>().ok());
        let lon = crate::db::repos::settings::SettingsRepo::get(&self.db, "longitude")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<f64>().ok());

        if let (Some(lat), Some(lon)) = (lat, lon) {
            let address = crate::tools::geo_utils::reverse_geocode(lat, lon).await?;
            // Cache back to settings so next call is instant
            let _ =
                crate::db::repos::settings::SettingsRepo::set(&self.db, "location_name", &address)
                    .await;
            Some(address)
        } else {
            None
        }
    }

    /// Streaming entry point: runs the ReAct loop and emits [`SSEEvent`] values
    /// over the provided channel so the frontend can receive them incrementally.
    pub async fn process_message_stream(
        &self,
        profile_id: &str,
        user_message: &str,
        browser_context: Option<BrowserContext>,
        tx: mpsc::Sender<SSEEvent>,
    ) -> Result<(), AgentError> {
        tracing::info!(
            user_message_len = %user_message.len(),
            "🚀 Orchestrator processing message stream"
        );

        let mut iterations = 0usize;
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut used_tools: Vec<String> = Vec::new();
        let mut tool_call_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // 1. Classify intent
        let classification = self.classifier.classify(user_message);

        // 2. Build context
        let ctx = self
            .context_builder
            .build(classification.strategy.clone(), profile_id, user_message)
            .await?;

        // Read settings from DB (max_window_tokens, system_prompt)
        let max_window_tokens =
            crate::db::repos::settings::SettingsRepo::get(&self.db, "max_window_tokens")
                .await?
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(10000);

        // The system prompt is stored in the `settings` table (seeded by
        // migration). If it is missing or empty, fall back to a minimal prompt.
        let system_prompt =
            match crate::db::repos::settings::SettingsRepo::get(&self.db, "system_prompt").await? {
                Some(p) if !p.trim().is_empty() => p,
                _ => {
                    tracing::warn!(
                        "settings.system_prompt is missing or empty; using minimal fallback"
                    );
                    DEFAULT_SYSTEM_PROMPT_FALLBACK.to_string()
                }
            };

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
            let fecha = format_browser_timestamp(&ctx.timestamp, &ctx.timezone)
                .unwrap_or_else(|| ctx.timestamp.clone());

            let mut parts = vec![format!("{:}.", fecha)];

            // Reverse‑geocode coordinates if we have them but no location name yet
            let location_name = if let (Some(lat), Some(lon)) = (ctx.latitude, ctx.longitude) {
                match &ctx.location_name {
                    Some(name) => Some(name.clone()),
                    None => reverse_geocode(lat, lon).await,
                }
            } else {
                None
            };

            if let (Some(lat), Some(lon)) = (ctx.latitude, ctx.longitude) {
                match &location_name {
                    Some(name) => {
                        parts.push(format!("Ubicación: {} ({:.4}, {:.4}).", name, lat, lon))
                    }
                    None => parts.push(format!("Coordenadas: ({:.4}, {:.4}).", lat, lon)),
                }
            }

            tracing::info!(
                timestamp = %ctx.timestamp,
                timezone = %ctx.timezone,
                latitude = ?ctx.latitude,
                longitude = ?ctx.longitude,
                location_name = ?location_name,
                "🌍 Browser context injected"
            );

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
            let history = crate::db::repos::messages::MessagesRepo::list_by_token_budget(
                &self.db,
                max_window_tokens,
            )
            .await?;
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
        let location = self.resolve_location().await;
        let user_message_id = {
            let collapse_callback = self
                .collapse_tx
                .clone()
                .map(crate::workers::collapse::collapse_forwarder);
            let msg = crate::db::repos::messages::MessagesRepo::create(
                &self.db,
                "user",
                user_message,
                None,
                None,
                location.as_deref(),
                None, // tools_used (user messages don't have this)
                self.config.collapse_threshold_tokens,
                collapse_callback,
            )
            .await?;
            msg.id
        };

        // Notify the episodic memory worker about the new user message
        if let Some(ref tx) = self.memory_tx {
            let _ = tx.send(()).await;
        }

        'react_loop: loop {
            if iterations >= self.config.max_iterations {
                tracing::error!(error = %AgentError::MaxIterationsExceeded, "❌ Orchestrator error");
                let _ = tx
                    .send(SSEEvent::Error {
                        message: "Max iterations exceeded".into(),
                    })
                    .await
                    .ok();
                return Err(AgentError::MaxIterationsExceeded);
            }

            tracing::debug!(iteration = %iterations, "ReAct loop iteration");

            let request = ChatRequest {
                model: self.config.model.clone(),
                messages: messages.clone(),
                tools: Some(self.registry.definitions()),
                temperature: None,
                max_tokens: Some(self.config.max_tokens_per_turn),
                stream: true,
            };

            let request_body_str = serde_json::to_string(&request).unwrap_or_default();
            let mut stream = self.llm.chat_stream(request).await?;
            let stream_start = std::time::Instant::now();

            iterations += 1;

            let mut content_buffer = String::new();
            let mut tool_calls_from_stream: Option<Vec<ToolCall>> = None;

            tracing::debug!(
                iteration = %iterations,
                "LLM stream started"
            );

            while let Some(event) = stream.next().await {
                let event = event.map_err(|e| AgentError::LLMError(e.to_string()))?;
                match event {
                    StreamEvent::Chunk(text) => {
                        content_buffer.push_str(&text);
                        if tx.send(SSEEvent::Chunk { content: text }).await.is_err() {
                            return Ok(()); // client disconnected
                        }
                    }
                    StreamEvent::ToolCall(tc) => {
                        tool_calls_from_stream.get_or_insert_with(Vec::new).push(tc);
                    }
                    StreamEvent::Done(response) => {
                        // Prefer tool_calls from Done (full arguments from finalize())
                        let tool_calls = response
                            .message
                            .tool_calls
                            .clone()
                            .or_else(|| tool_calls_from_stream.take());

                        // Record stats for this LLM call
                        let prompt_tokens = response
                            .usage
                            .as_ref()
                            .map(|u| u.prompt_tokens as i64)
                            .unwrap_or(0);
                        let completion_tokens = response
                            .usage
                            .as_ref()
                            .map(|u| u.completion_tokens as i64)
                            .unwrap_or(0);
                        let total_tokens = prompt_tokens + completion_tokens;

                        let _ = StatsRepo::record_request(
                            &self.db,
                            &Uuid::new_v4().to_string(),
                            &self.config.model,
                            Some(profile_id),
                            prompt_tokens,
                            completion_tokens,
                            total_tokens,
                            response
                                .usage
                                .as_ref()
                                .map(|u| u.cached_tokens as i64)
                                .unwrap_or(0),
                            response
                                .usage
                                .as_ref()
                                .map(|u| u.reasoning_tokens as i64)
                                .unwrap_or(0),
                            response.usage.as_ref().map(|u| u.cost).unwrap_or(0.0),
                            Some(stream_start.elapsed().as_millis() as i64),
                            "success",
                            None,
                            None,
                            None,
                        )
                        .await;

                        self.save_last_call(
                            &self.config.model,
                            Some(&request_body_str),
                            Some(&serde_json::to_string(&response).unwrap_or_default()),
                            prompt_tokens as u32,
                            completion_tokens as u32,
                            total_tokens as u32,
                            response
                                .usage
                                .as_ref()
                                .map(|u| u.cached_tokens)
                                .unwrap_or(0),
                            response
                                .usage
                                .as_ref()
                                .map(|u| u.reasoning_tokens)
                                .unwrap_or(0),
                            response.usage.as_ref().map(|u| u.cost).unwrap_or(0.0),
                            Some(stream_start.elapsed().as_millis() as i64),
                            "success",
                            None,
                            None,
                        );

                        if let Some(tcs) = tool_calls {
                            let content = content_buffer.clone();

                            tracing::debug!(tool_calls_from_stream = ?tool_calls_from_stream.as_ref().map(|t| t.len()), response_tool_calls = ?response.message.tool_calls.as_ref().map(|t| t.len()), "📦 Orchestrator tool_calls check");
                            tracing::debug!(
                                iteration = %iterations,
                                tool_call_count = %tcs.len(),
                                "Tool calls received from stream"
                            );

                            messages.push(ChatMessage {
                                role: "assistant".into(),
                                content,
                                tool_calls: Some(tcs.clone()),
                                tool_result: None,
                                tool_call_id: None,
                            });

                            for tc in &tcs {
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
                                let guardrail = self
                                    .guardrails
                                    .check(&tc.name, &tc.arguments)
                                    .map_err(|e| {
                                        tracing::error!(error = %e, "❌ Orchestrator error");
                                        AgentError::GuardrailError(e.to_string())
                                    })?;

                                match guardrail {
                                    GuardrailResult::Allowed { .. } => {
                                        // Check per-tool retry limit (max 3 calls per tool per ReAct loop)
                                        let op =
                                            tc.arguments.get("operation").and_then(|v| v.as_str());
                                        let op_key = match op {
                                            Some(op_val) => format!("{}::{}", tc.name, op_val),
                                            None => tc.name.clone(),
                                        };
                                        let tool_count =
                                            tool_call_counts.entry(op_key.clone()).or_insert(0);
                                        *tool_count += 1;
                                        if *tool_count > MAX_TOOL_RETRIES {
                                            let display_name = match op {
                                                Some(op_val) => format!("{}::{}", tc.name, op_val),
                                                None => tc.name.clone(),
                                            };
                                            tracing::warn!(
                                                tool_name = %display_name,
                                                call_count = %tool_count,
                                                "⚠️ Tool retry limit reached"
                                            );
                                            let msg = format!(
                                                "Tool '{}' has been called 3 times. No more retries allowed. Inform the user and suggest alternatives.",
                                                display_name
                                            );
                                            messages.push(ChatMessage {
                                                role: "tool".into(),
                                                content: msg.clone(),
                                                tool_calls: None,
                                                tool_result: None,
                                                tool_call_id: Some(tc.id.clone()),
                                            });
                                            // Still emit tool_result event so frontend knows tool was "called"
                                            let _ = tx
                                                .send(SSEEvent::ToolResult {
                                                    name: tc.name.clone(),
                                                    success: false,
                                                })
                                                .await
                                                .ok();
                                            continue;
                                        }

                                        tracing::debug!(
                                            tool_name = %tc.name,
                                            tool_args = %tc.arguments,
                                            "🔧 Executing tool"
                                        );

                                        // Inject profile_id from authenticated session
                                        let mut args = tc.arguments.clone();
                                        if let Some(obj) = args.as_object_mut() {
                                            obj.insert(
                                                "profile_id".into(),
                                                serde_json::json!(profile_id),
                                            );
                                        }

                                        let tool_result =
                                            match self.registry.execute(&tc.name, args).await {
                                                Ok(result) => result,
                                                Err(e) => {
                                                    tracing::error!(
                                                        tool_name = %tc.name,
                                                        tool_args = %tc.arguments,
                                                        error = %e,
                                                        error_debug = ?e,
                                                        "❌ Tool execution error"
                                                    );
                                                    ToolResult {
                                                        success: false,
                                                        data: serde_json::json!({}),
                                                        message: Some(e.to_string()),
                                                    }
                                                }
                                            };

                                        match tool_result.success {
                                            true => {
                                                // Track the tool name for the footer
                                                used_tools.push(op_key.clone());

                                                // Emit success event
                                                let _ = tx
                                                    .send(SSEEvent::ToolResult {
                                                        name: tc.name.clone(),
                                                        success: true,
                                                    })
                                                    .await
                                                    .ok();

                                                messages.push(ChatMessage {
                                                    role: "tool".into(),
                                                    content: serde_json::to_string(
                                                        &tool_result.data,
                                                    )
                                                    .unwrap_or_default(),
                                                    tool_calls: None,
                                                    tool_result: Some(tool_result.data),
                                                    tool_call_id: Some(tc.id.clone()),
                                                });
                                            }
                                            false => {
                                                let err_msg =
                                                    tool_result.message.unwrap_or_default();
                                                tracing::error!(
                                                    tool_name = %tc.name,
                                                    tool_args = %tc.arguments,
                                                    error_msg = %err_msg,
                                                    "❌ Tool returned failure"
                                                );
                                                let _ = tx
                                                    .send(SSEEvent::ToolResult {
                                                        name: tc.name.clone(),
                                                        success: false,
                                                    })
                                                    .await
                                                    .ok();

                                                messages.push(ChatMessage {
                                                    role: "tool".into(),
                                                    content: format!("Error: {}", err_msg),
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
                                            .await
                                            .ok();

                                        let err = AgentError::GuardrailError(format!(
                                            "Tool '{}' requires explicit approval (request_id: {})",
                                            tc.name, request_id
                                        ));
                                        tracing::error!(error = %err, "❌ Orchestrator error");
                                        return Err(err);
                                    }
                                }
                            }

                            content_buffer.clear();
                            continue 'react_loop;
                        } else {
                            // No tool calls — final answer

                            // Build tools_used string with counts
                            let tools_used: Option<String> = if !used_tools.is_empty() {
                                let mut counts: std::collections::HashMap<String, usize> =
                                    std::collections::HashMap::new();
                                for t in &used_tools {
                                    *counts.entry(t.clone()).or_insert(0) += 1;
                                }
                                let mut parts: Vec<String> = Vec::new();
                                // Sort for deterministic output
                                let mut keys: Vec<&String> = counts.keys().collect();
                                keys.sort();
                                for key in keys {
                                    let count = counts[key];
                                    if count > 1 {
                                        parts.push(format!("({}) {}", count, key));
                                    } else {
                                        parts.push(key.clone());
                                    }
                                }
                                Some(parts.join(", "))
                            } else {
                                None
                            };

                            // Persist assistant message to DB (capture the real UUID)
                            let location = self.resolve_location().await;
                            let assistant_message_id = {
                                let collapse_callback = self
                                    .collapse_tx
                                    .clone()
                                    .map(crate::workers::collapse::collapse_forwarder);
                                let msg = crate::db::repos::messages::MessagesRepo::create(
                                    &self.db,
                                    "assistant",
                                    &content_buffer,
                                    None,
                                    None,
                                    location.as_deref(),
                                    tools_used.as_deref(),
                                    self.config.collapse_threshold_tokens,
                                    collapse_callback,
                                )
                                .await?;
                                msg.id
                            };

                            // Notify the episodic memory worker about the new assistant message
                            if let Some(ref tx) = self.memory_tx {
                                let _ = tx.send(()).await;
                            }

                            // Fetch user message metadata for the frontend
                            let (user_location, user_created_at) = {
                                crate::db::repos::messages::MessagesRepo::find_by_id(
                                    &self.db,
                                    &user_message_id,
                                )
                                .await
                                .ok()
                                .flatten()
                                .map(|msg| (msg.location, msg.created_at))
                                .unwrap_or_else(|| (None, String::new()))
                            };

                            let _ = tx
                                .send(SSEEvent::Done {
                                    message_id: assistant_message_id,
                                    user_message_id: user_message_id.clone(),
                                    location: location.clone(),
                                    tools_used: tools_used.clone(),
                                    user_location,
                                    user_created_at,
                                })
                                .await
                                .ok();

                            tracing::info!(
                                iterations,
                                "✅ Orchestrator finished processing message"
                            );

                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reflection analyzer
// ---------------------------------------------------------------------------

pub struct ReflectionAnalyzer {
    llm: Arc<dyn LLMProvider>,
    last_api_call: Arc<RwLock<Option<LastApiCall>>>,
}

impl ReflectionAnalyzer {
    pub fn new(llm: Arc<dyn LLMProvider>, last_api_call: Arc<RwLock<Option<LastApiCall>>>) -> Self {
        Self { llm, last_api_call }
    }

    #[allow(clippy::too_many_arguments)]
    fn save_last_call(
        &self,
        model: &str,
        request_body: Option<&str>,
        response_body: Option<&str>,
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
        cached_tokens: u32,
        reasoning_tokens: u32,
        cost: f64,
        duration_ms: Option<i64>,
        status: &str,
        error_message: Option<&str>,
        tool_calls: Option<&str>,
    ) {
        let last = LastApiCall {
            model: model.to_string(),
            request_body: request_body.map(|s| s.to_string()),
            response_body: response_body.map(|s| s.to_string()),
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cached_tokens,
            reasoning_tokens,
            cost,
            duration_ms,
            status: status.to_string(),
            error_message: error_message.map(|s| s.to_string()),
            tool_calls: tool_calls.map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        *self.last_api_call.write().unwrap() = Some(last);
    }

    /// Ask the LLM to reflect on its own response — checking coherence,
    /// completeness, and whether clarification is needed.
    pub async fn analyze(
        &self,
        conversation: &[ChatMessage],
        response: &str,
        db: &SqlitePool,
        profile_id: &str,
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

        let request_body_str = serde_json::to_string(&request).unwrap_or_default();
        let start = Instant::now();
        match self.llm.chat(request).await {
            Ok(resp) => {
                // Record stats for this LLM call
                let duration_ms = start.elapsed().as_millis() as i64;
                let prompt_tokens = resp
                    .usage
                    .as_ref()
                    .map(|u| u.prompt_tokens as i64)
                    .unwrap_or(0);
                let completion_tokens = resp
                    .usage
                    .as_ref()
                    .map(|u| u.completion_tokens as i64)
                    .unwrap_or(0);
                let total_tokens = prompt_tokens + completion_tokens;
                let _ = StatsRepo::record_request(
                    db,
                    &Uuid::new_v4().to_string(),
                    "default",
                    Some(profile_id),
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                    resp.usage
                        .as_ref()
                        .map(|u| u.cached_tokens as i64)
                        .unwrap_or(0),
                    resp.usage
                        .as_ref()
                        .map(|u| u.reasoning_tokens as i64)
                        .unwrap_or(0),
                    resp.usage.as_ref().map(|u| u.cost).unwrap_or(0.0),
                    Some(duration_ms),
                    "success",
                    None,
                    None,
                    None,
                )
                .await;

                self.save_last_call(
                    "default",
                    Some(&request_body_str),
                    Some(&serde_json::to_string(&resp).unwrap_or_default()),
                    prompt_tokens as u32,
                    completion_tokens as u32,
                    total_tokens as u32,
                    resp.usage.as_ref().map(|u| u.cached_tokens).unwrap_or(0),
                    resp.usage.as_ref().map(|u| u.reasoning_tokens).unwrap_or(0),
                    resp.usage.as_ref().map(|u| u.cost).unwrap_or(0.0),
                    Some(duration_ms),
                    "success",
                    None,
                    None,
                );

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
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as i64;
                let _ = StatsRepo::record_request(
                    db,
                    &Uuid::new_v4().to_string(),
                    "default",
                    Some(profile_id),
                    0,
                    0,
                    0,
                    0,
                    0,
                    0.0,
                    Some(duration_ms),
                    "error",
                    Some(&e.to_string()),
                    None,
                    None,
                )
                .await;
                self.save_last_call(
                    "default",
                    None,
                    None,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0.0,
                    Some(duration_ms),
                    "error",
                    Some(&e.to_string()),
                    None,
                );
                Err(AgentError::LLMError(e.to_string()))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::{ChatResponse, LLMError, StreamEvent, TokenUsage};
    use crate::tools::permission::Permission;
    use crate::tools::r#trait::{Tool, ToolError, ToolResult};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
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
            location: None,
            tools_used: None,
            user_location: None,
            user_created_at: String::new(),
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
            location: None,
            tools_used: None,
            user_location: None,
            user_created_at: String::new(),
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
    }

    #[test]
    fn test_default_system_prompt_fallback_is_minimal_and_non_empty() {
        // The hardcoded personality template was removed: the fallback must be
        // a minimal generic prompt.
        assert!(!DEFAULT_SYSTEM_PROMPT_FALLBACK.is_empty());
        assert_eq!(
            DEFAULT_SYSTEM_PROMPT_FALLBACK,
            "You are Valet, a helpful AI assistant."
        );
    }

    /// Build an orchestrator backed by a DB and a mock LLM that captures the
    /// first system message of every request.
    async fn build_orchestrator_with_capture(
        pool: SqlitePool,
        captured: Arc<Mutex<Option<String>>>,
        enable_reflection: bool,
    ) -> Orchestrator {
        let llm = Arc::new(SystemPromptCaptureLLM { captured });
        let registry = Arc::new(crate::tools::registry::ToolRegistry::new());
        let guardrails = Arc::new(crate::orchestrator::guardrails::Guardrails::new(
            registry.clone(),
        ));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig {
            enable_reflection,
            ..Default::default()
        };
        Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            pool,
            None,
            None,
            Arc::new(RwLock::new(None)),
        )
    }

    #[tokio::test]
    async fn test_system_prompt_fallback_when_missing_in_stream(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;
        // Remove the value seeded by the migration to force the fallback.
        crate::db::repos::settings::SettingsRepo::delete(&pool, "system_prompt").await?;

        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let orchestrator =
            build_orchestrator_with_capture(pool.clone(), captured.clone(), false).await;

        let (tx, mut rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("profile-1", "Hola", None, tx)
            .await?;
        while rx.recv().await.is_some() {}

        assert_eq!(
            captured.lock().unwrap().as_deref(),
            Some(DEFAULT_SYSTEM_PROMPT_FALLBACK),
            "When settings.system_prompt is missing the minimal fallback must be used"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_process_message_uses_system_prompt_from_db(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;
        crate::db::repos::settings::SettingsRepo::set(&pool, "system_prompt", "Prompt de prueba")
            .await?;

        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        // Disable reflection so only the main ReAct call captures the prompt.
        let orchestrator =
            build_orchestrator_with_capture(pool.clone(), captured.clone(), false).await;

        orchestrator.process_message("profile-1", "Hola").await?;

        assert_eq!(
            captured.lock().unwrap().as_deref(),
            Some("Prompt de prueba"),
            "process_message must use settings.system_prompt when present"
        );

        Ok(())
    }

    // ─── Characterization: memory injection into the LLM request (task 1.2) ──
    //
    // These tests pin down CURRENT behaviour: episodic memory is injected as a
    // `system` message prefixed with the literal `"[Memory context] "`, and it
    // only ever appears on the `RAG` path (`!doc ...`), because only that
    // strategy produces memories inside `ContextBuilder`. On the common
    // `SlidingWindow` path no `[Memory context]` reaches the LLM today.
    //
    // The `episodic-memory-injection` change will deliberately break the
    // SlidingWindow half of this contract (memory becomes strategy-independent)
    // and replace the literal with a composed `<episodic_memory>` block. See
    // the invariant exceptions documented in `context_builder`'s tests.

    /// Mock LLM that captures the full message list of the *first* request.
    struct FullRequestCaptureLLM {
        captured: Arc<Mutex<Option<Vec<ChatMessage>>>>,
    }

    #[async_trait::async_trait]
    impl LLMProvider for FullRequestCaptureLLM {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
            {
                let mut slot = self.captured.lock().unwrap();
                if slot.is_none() {
                    *slot = Some(request.messages.clone());
                }
            }
            Ok(ChatResponse {
                message: ChatMessage {
                    role: "assistant".into(),
                    content: "OK.".into(),
                    tool_calls: None,
                    tool_result: None,
                    tool_call_id: None,
                },
                usage: None,
            })
        }

        async fn chat_stream(
            &self,
            request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            {
                let mut slot = self.captured.lock().unwrap();
                if slot.is_none() {
                    *slot = Some(request.messages.clone());
                }
            }
            let events: Vec<Result<StreamEvent, LLMError>> =
                vec![Ok(StreamEvent::Done(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "OK.".into(),
                        tool_calls: None,
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                }))];
            Ok(Box::pin(futures::stream::iter(events)))
        }
    }

    /// Fixed embedding provider (dimension 1024, as `vec0` declares),
    /// matching the seeded vector.
    struct FixedEmbedProvider;

    /// Pad a leading slice to the 1024 dimensions the `vec0` table declares.
    fn v1024(leading: &[f32]) -> Vec<f32> {
        let mut v = leading.to_vec();
        v.resize(1024, 0.0);
        v
    }

    #[async_trait::async_trait]
    impl crate::embeddings::EmbeddingProvider for FixedEmbedProvider {
        async fn embed(
            &self,
            _input: &str,
        ) -> Result<Vec<f32>, crate::embeddings::provider::EmbeddingError> {
            Ok(v1024(&[0.1, 0.2, 0.3]))
        }
    }

    /// Seed one `memory` card plus its `vec_memory` row.
    ///
    /// CHANGED ON PURPOSE (invariant exception 2): `vec_memory` now stores
    /// binary `vec0` vectors of the declared 1024 dimensions, so the row is
    /// written through `vec_f32(?)` instead of as JSON text. The
    /// characterization assertions are untouched.
    async fn seed_one_memory(pool: &SqlitePool) {
        let mem = crate::db::repos::memory::MemoryRepo::create(
            pool,
            "User likes Rust",
            10,
            &serde_json::json!({"tags": ["rust", "backend"]}),
        )
        .await
        .expect("create memory should succeed");
        let json = serde_json::to_string(&v1024(&[0.1, 0.2, 0.3])).expect("serialize embedding");
        sqlx::query("INSERT INTO vec_memory (id, embedding) VALUES (?1, vec_f32(?2))")
            .bind(&mem.id)
            .bind(&json)
            .execute(pool)
            .await
            .expect("insert vec_memory row");
    }

    /// Builder wired with pool + provider and one seeded memory, so the RAG
    /// path can actually retrieve something.
    fn memory_context_builder(pool: SqlitePool) -> ContextBuilder {
        ContextBuilder {
            pool: Some(pool),
            provider: Some(Arc::new(FixedEmbedProvider)),
            rag_budget_tokens: 2000,
        }
    }

    /// Orchestrator with a full-request capturer and no reflection, so the
    /// first (only) main LLM call is the one observed.
    async fn build_orchestrator_with_full_capture(
        pool: SqlitePool,
        captured: Arc<Mutex<Option<Vec<ChatMessage>>>>,
        context_builder: ContextBuilder,
    ) -> Orchestrator {
        let llm = Arc::new(FullRequestCaptureLLM { captured });
        let registry = Arc::new(crate::tools::registry::ToolRegistry::new());
        let guardrails = Arc::new(crate::orchestrator::guardrails::Guardrails::new(
            registry.clone(),
        ));
        let config = OrchestratorConfig {
            enable_reflection: false,
            ..Default::default()
        };
        Orchestrator::new(
            llm,
            registry,
            guardrails,
            Arc::new(context_builder),
            config,
            pool,
            None,
            None,
            Arc::new(RwLock::new(None)),
        )
    }

    /// CURRENT: on the RAG path (`!doc`), the LLM request carries a `system`
    /// message prefixed with `"[Memory context] "`.
    #[tokio::test]
    async fn characterization_rag_injects_memory_context_in_process_message(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;
        seed_one_memory(&pool).await;

        let captured: Arc<Mutex<Option<Vec<ChatMessage>>>> = Arc::new(Mutex::new(None));
        let orchestrator = build_orchestrator_with_full_capture(
            pool.clone(),
            captured.clone(),
            memory_context_builder(pool.clone()),
        )
        .await;

        orchestrator
            .process_message("profile-1", "!doc what does the user like")
            .await?;

        let messages = captured
            .lock()
            .unwrap()
            .clone()
            .expect("LLM must have been called once");
        let injected = messages
            .iter()
            .filter(|m| m.role == "system" && m.content.starts_with("[Memory context] "))
            .collect::<Vec<_>>();
        assert_eq!(
            injected.len(),
            1,
            "CURRENT behaviour: RAG injects exactly one `[Memory context] ` system message"
        );
        assert!(
            injected[0].content.contains("User likes Rust"),
            "the injected memory should carry the stored content, got: {:?}",
            injected[0].content
        );
        Ok(())
    }

    /// CURRENT: the same holds for the streaming path (`process_message_stream`).
    #[tokio::test]
    async fn characterization_rag_injects_memory_context_in_stream(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;
        seed_one_memory(&pool).await;

        let captured: Arc<Mutex<Option<Vec<ChatMessage>>>> = Arc::new(Mutex::new(None));
        let orchestrator = build_orchestrator_with_full_capture(
            pool.clone(),
            captured.clone(),
            memory_context_builder(pool.clone()),
        )
        .await;

        let (tx, mut rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("profile-1", "!doc what does the user like", None, tx)
            .await?;
        while rx.recv().await.is_some() {}

        let messages = captured
            .lock()
            .unwrap()
            .clone()
            .expect("LLM must have been called once");
        let injected = messages
            .iter()
            .filter(|m| m.role == "system" && m.content.starts_with("[Memory context] "))
            .collect::<Vec<_>>();
        assert_eq!(
            injected.len(),
            1,
            "CURRENT behaviour: streaming RAG injects one `[Memory context] ` system message"
        );
        assert!(injected[0].content.contains("User likes Rust"));
        Ok(())
    }

    /// CURRENT: the common `SlidingWindow` path receives **no** memory, even
    /// though the builder holds a pool, a provider and a matching memory.
    /// This is exactly what the change will alter.
    #[tokio::test]
    async fn characterization_sliding_window_does_not_inject_memory_in_process_message(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;
        seed_one_memory(&pool).await;

        let captured: Arc<Mutex<Option<Vec<ChatMessage>>>> = Arc::new(Mutex::new(None));
        let orchestrator = build_orchestrator_with_full_capture(
            pool.clone(),
            captured.clone(),
            memory_context_builder(pool.clone()),
        )
        .await;

        orchestrator.process_message("profile-1", "Hola").await?;

        let messages = captured
            .lock()
            .unwrap()
            .clone()
            .expect("LLM must have been called once");
        assert!(
            !messages
                .iter()
                .any(|m| m.content.contains("[Memory context]")),
            "CURRENT behaviour: SlidingWindow injects no memory (this will change)"
        );
        Ok(())
    }

    /// CURRENT: same for the streaming path on `SlidingWindow`.
    #[tokio::test]
    async fn characterization_sliding_window_does_not_inject_memory_in_stream(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;
        seed_one_memory(&pool).await;

        let captured: Arc<Mutex<Option<Vec<ChatMessage>>>> = Arc::new(Mutex::new(None));
        let orchestrator = build_orchestrator_with_full_capture(
            pool.clone(),
            captured.clone(),
            memory_context_builder(pool.clone()),
        )
        .await;

        let (tx, mut rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("profile-1", "Hola", None, tx)
            .await?;
        while rx.recv().await.is_some() {}

        let messages = captured
            .lock()
            .unwrap()
            .clone()
            .expect("LLM must have been called once");
        assert!(
            !messages
                .iter()
                .any(|m| m.content.contains("[Memory context]")),
            "CURRENT behaviour: streaming SlidingWindow injects no memory (this will change)"
        );
        Ok(())
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
            request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            let result = self.chat(request).await?;
            let content = result.message.content.clone();
            let tool_calls = result.message.tool_calls.clone();

            let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();

            if let Some(tcs) = tool_calls {
                for tc in tcs {
                    events.push(Ok(StreamEvent::ToolCall(tc)));
                }
                events.push(Ok(StreamEvent::Done(result)));
            } else {
                for chunk in content
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(10)
                    .map(|c| c.iter().collect::<String>())
                {
                    events.push(Ok(StreamEvent::Chunk(chunk)));
                }
                events.push(Ok(StreamEvent::Done(result)));
            }

            let stream = futures::stream::iter(events);
            Ok(Box::pin(stream))
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
    async fn test_process_message_stream_adds_tool_footer() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::db::repos::messages::MessagesRepo;

        // 1. Create in-memory SQLite pool and run migrations
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Create a registry with a mock weather tool
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockWeatherTool));
        let registry = Arc::new(registry);

        // 3. Create mock LLM that returns tool call then answer
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
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        // 4. Call process_message_stream (no conversation_id needed)
        let (tx, _rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("profile-id", "What's the weather?", None, tx)
            .await?;

        // 5. Query the DB for the assistant message and verify footer
        let (messages, _) = MessagesRepo::list_all(&pool, 100, None).await?;
        let assistant_messages: Vec<_> =
            messages.iter().filter(|m| m.role == "assistant").collect();

        assert!(
            !assistant_messages.is_empty(),
            "Expected at least one assistant message in DB"
        );

        let last_msg = assistant_messages.last().unwrap();
        assert!(
            last_msg.tools_used.is_some(),
            "Assistant message should have tools_used set, got: {:?}",
            last_msg.tools_used
        );
        assert!(
            last_msg.tools_used.as_deref().unwrap().contains("weather"),
            "tools_used should contain 'weather', got: {:?}",
            last_msg.tools_used
        );
        assert!(
            last_msg
                .content
                .contains("The weather in Madrid is 22°C and sunny."),
            "Assistant message should contain the original content, got: {}",
            last_msg.content
        );
        Ok(())
    }

    /// Given an Orchestrator processing a long user message (8000 chars),
    /// when the message is persisted via MessagesRepo::create(),
    /// then the collapse callback SHOULD fire and send the message_id through
    /// the collapse channel.
    ///
    /// RED: This test will fail because the orchestrator currently passes
    /// `None` as the `on_collapse_needed` callback to MessagesRepo::create(),
    /// so no message_id arrives on collapse_rx.
    #[tokio::test]
    async fn test_collapse_callback_fires_for_long_user_message(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory SQLite pool and run migrations
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Create a simple mock LLM that returns plain text
        struct SimpleMockLLM;

        #[async_trait::async_trait]
        impl LLMProvider for SimpleMockLLM {
            async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "This is a simple response to a very long message.".into(),
                        tool_calls: None,
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            }

            async fn chat_stream(
                &self,
                request: ChatRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
            {
                let result = self.chat(request).await?;
                let content = result.message.content.clone();
                let tool_calls = result.message.tool_calls.clone();

                let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();

                if let Some(tcs) = tool_calls {
                    for tc in tcs {
                        events.push(Ok(StreamEvent::ToolCall(tc)));
                    }
                    events.push(Ok(StreamEvent::Done(result)));
                } else {
                    for chunk in content
                        .chars()
                        .collect::<Vec<_>>()
                        .chunks(10)
                        .map(|c| c.iter().collect::<String>())
                    {
                        events.push(Ok(StreamEvent::Chunk(chunk)));
                    }
                    events.push(Ok(StreamEvent::Done(result)));
                }

                let stream = futures::stream::iter(events);
                Ok(Box::pin(stream))
            }
        }

        // 3. Create collapse channel that should receive the message_id
        let (collapse_tx, mut collapse_rx) = mpsc::channel::<String>(16);

        // 4. Create orchestrator
        let llm = Arc::new(SimpleMockLLM);
        let registry = Arc::new(ToolRegistry::new());
        let guardrails = Arc::new(Guardrails::new(registry.clone()));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            pool.clone(),
            Some(collapse_tx),
            None,
            Arc::new(RwLock::new(None)),
        );

        // 5. Call process_message_stream with a very long message (2000+ words for ~2660 tokens)
        let long_msg = "x ".repeat(2000);
        let (tx, _rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("profile-id", &long_msg, None, tx)
            .await?;

        // 6. Verify that the collapse channel received the message_id
        let received =
            tokio::time::timeout(std::time::Duration::from_millis(500), collapse_rx.recv()).await;

        match received {
            Ok(Some(msg_id)) => {
                assert!(!msg_id.is_empty(), "message_id should not be empty");
            }
            _ => {
                panic!("Should have received message_id via collapse channel for long message");
            }
        }
        Ok(())
    }

    /// Given an Orchestrator with memory_tx wired,
    /// when process_message_stream persists a user message,
    /// then a signal (()) SHOULD be sent on memory_tx.
    ///
    /// RED: This test will fail because the orchestrator does not yet
    /// send signals through memory_tx after persisting messages.
    #[tokio::test]
    async fn test_orchestrator_sends_memory_signal_on_user_message(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory SQLite pool and run migrations
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Create a local mock LLM that returns plain text immediately
        struct MemoryMockLLM;

        #[async_trait::async_trait]
        impl LLMProvider for MemoryMockLLM {
            async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Simple response for memory test.".into(),
                        tool_calls: None,
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            }

            async fn chat_stream(
                &self,
                request: ChatRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
            {
                let result = self.chat(request).await?;
                let content = result.message.content.clone();

                let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();
                for chunk in content
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(10)
                    .map(|c| c.iter().collect::<String>())
                {
                    events.push(Ok(StreamEvent::Chunk(chunk)));
                }
                events.push(Ok(StreamEvent::Done(result)));

                let stream = futures::stream::iter(events);
                Ok(Box::pin(stream))
            }
        }

        let llm = Arc::new(MemoryMockLLM);
        let registry = Arc::new(ToolRegistry::new());
        let guardrails = Arc::new(Guardrails::new(registry.clone()));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        // 3. Create memory_tx channel
        let (memory_tx, mut memory_rx) = mpsc::channel::<()>(16);

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            pool.clone(),
            None,
            Some(memory_tx),
            Arc::new(RwLock::new(None)),
        );

        // 4. Call process_message_stream
        let (tx, _rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("test-profile", "Hello", None, tx)
            .await?;

        // 5. Verify that memory_rx receives at least one signal
        let received =
            tokio::time::timeout(std::time::Duration::from_millis(500), memory_rx.recv()).await;

        match received {
            Ok(Some(())) => { /* expected: received signal */ }
            _ => {
                panic!("Should have received () via memory_tx after persisting user message");
            }
        }
        Ok(())
    }

    /// Given an Orchestrator with memory_tx wired,
    /// when process_message_stream persists both user and assistant messages,
    /// then TWO signals SHOULD be sent on memory_tx
    /// (one for the user message, one for the assistant response).
    ///
    /// RED: This test will fail because the orchestrator does not yet
    /// send signals through memory_tx after persisting messages.
    #[tokio::test]
    async fn test_orchestrator_sends_memory_signal_on_assistant_message(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory SQLite pool and run migrations
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Create a local mock LLM that returns plain text immediately
        struct MemoryMockLLM;

        #[async_trait::async_trait]
        impl LLMProvider for MemoryMockLLM {
            async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Simple response for memory test.".into(),
                        tool_calls: None,
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            }

            async fn chat_stream(
                &self,
                request: ChatRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
            {
                let result = self.chat(request).await?;
                let content = result.message.content.clone();

                let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();
                for chunk in content
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(10)
                    .map(|c| c.iter().collect::<String>())
                {
                    events.push(Ok(StreamEvent::Chunk(chunk)));
                }
                events.push(Ok(StreamEvent::Done(result)));

                let stream = futures::stream::iter(events);
                Ok(Box::pin(stream))
            }
        }

        let llm = Arc::new(MemoryMockLLM);
        let registry = Arc::new(ToolRegistry::new());
        let guardrails = Arc::new(Guardrails::new(registry.clone()));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        // 3. Create memory_tx channel
        let (memory_tx, mut memory_rx) = mpsc::channel::<()>(16);

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            pool.clone(),
            None,
            Some(memory_tx),
            Arc::new(RwLock::new(None)),
        );

        // 4. Call process_message_stream
        let (tx, _rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("test-profile", "Hello", None, tx)
            .await?;

        // 5. Verify that memory_rx receives at least TWO signals
        //    (user message + assistant response)
        let signal1 =
            tokio::time::timeout(std::time::Duration::from_millis(500), memory_rx.recv()).await;
        let signal2 =
            tokio::time::timeout(std::time::Duration::from_millis(500), memory_rx.recv()).await;

        match signal1 {
            Ok(Some(())) => { /* first signal received */ }
            _ => {
                panic!("Should have received first () via memory_tx (user message)");
            }
        }

        match signal2 {
            Ok(Some(())) => { /* second signal received */ }
            _ => {
                panic!("Should have received second () via memory_tx (assistant response)");
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Mock tool that always fails — used to test error recovery in ReAct loop
    // -----------------------------------------------------------------------

    struct MockFailingTool;

    #[async_trait::async_trait]
    impl Tool for MockFailingTool {
        fn name(&self) -> &'static str {
            "failing_tool"
        }

        fn description(&self) -> &'static str {
            "A tool that always fails"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        fn permission(&self) -> Permission {
            Permission::NoConfirm
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolError> {
            Err(ToolError::ExecutionError(
                "Overpass timeout simulated".into(),
            ))
        }
    }

    /// Mock LLM that returns a tool call for `failing_tool` on first
    /// invocation, then returns a plain-text answer on the second call.
    /// This exercises the scenario where a tool *errors* and the LLM
    /// should still get a chance to respond.
    struct MockLLMWithFailingToolThenAnswer {
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl LLMProvider for MockLLMWithFailingToolThenAnswer {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            if *count == 1 {
                // First call: return a tool call for the failing tool
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Let me look that up.".into(),
                        tool_calls: Some(vec![ToolCall {
                            id: "call-fail-1".into(),
                            name: "failing_tool".into(),
                            arguments: serde_json::json!({"query": "test"}),
                        }]),
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            } else {
                // Second call: return final answer (LLM recovers from error)
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "The tool failed, but I can still help.".into(),
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
            request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            let result = self.chat(request).await?;
            let content = result.message.content.clone();
            let tool_calls = result.message.tool_calls.clone();

            let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();

            if let Some(tcs) = tool_calls {
                for tc in tcs {
                    events.push(Ok(StreamEvent::ToolCall(tc)));
                }
                events.push(Ok(StreamEvent::Done(result)));
            } else {
                for chunk in content
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(10)
                    .map(|c| c.iter().collect::<String>())
                {
                    events.push(Ok(StreamEvent::Chunk(chunk)));
                }
                events.push(Ok(StreamEvent::Done(result)));
            }

            let stream = futures::stream::iter(events);
            Ok(Box::pin(stream))
        }
    }

    // -----------------------------------------------------------------------
    // Test: tool error recovery — currently RED because `?` breaks the loop
    // -----------------------------------------------------------------------
    //
    // Este test demuestra el comportamiento ACTUAL (roto): cuando un tool
    // falla con Err(ToolError), el `?` en la línea 670 propaga el error y
    // cortocircuita el ReAct loop, impidiendo que se emitan los eventos SSE
    // ToolResult { success: false }, Chunk y Done.
    //
    // El test captura los eventos SSE y verifica que se complete el flujo
    // completo (ToolCall -> ToolResult(success:false) -> Chunk -> Done).
    // Actualmente FALLA porque el error se propaga antes de emitir Done.
    //
    // RED: Este test falla → lo haremos pasar en GREEN.

    #[tokio::test]
    async fn test_tool_error_does_not_break_react_loop() -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory SQLite pool and run migrations
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Create a registry with a mock failing tool
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockFailingTool));
        let registry = Arc::new(registry);

        // 3. Create mock LLM that returns failing tool call then answer
        let llm = Arc::new(MockLLMWithFailingToolThenAnswer {
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
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        // 4. Call process_message_stream and capture SSE events
        let (tx, mut rx) = mpsc::channel(100);
        let result = orchestrator
            .process_message_stream("profile-id", "Look something up", None, tx)
            .await;

        // 5. Collect all SSE events with a timeout
        let mut events: Vec<SSEEvent> = Vec::new();
        while let Ok(Some(event)) =
            tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
        {
            events.push(event);
        }

        // 6. Verify the event sequence — this SHOULD work once the `?` is
        // replaced with a match that converts the error into a ToolResult.
        //
        // ACTUAL: This assertion fails because process_message_stream returns
        // Err(...) (the `?` propagates the ToolError) and no ToolResult event
        // is emitted for the failing tool.
        assert!(
            result.is_ok(),
            "El orquestador NO debe propagar errores de tool como errores del ReAct loop. \
             Error actual: {:?}",
            result.err()
        );

        // Verify the expected event sequence
        let tool_call_events: Vec<&SSEEvent> = events
            .iter()
            .filter(|e| matches!(e, SSEEvent::ToolCall { .. }))
            .collect();
        assert!(
            !tool_call_events.is_empty(),
            "Debe emitirse al menos un SSEEvent::ToolCall para failing_tool"
        );

        let tool_result_events: Vec<&SSEEvent> = events
            .iter()
            .filter(|e| matches!(e, SSEEvent::ToolResult { .. }))
            .collect();
        assert!(
            !tool_result_events.is_empty(),
            "Debe emitirse al menos un SSEEvent::ToolResult (incluyendo success: false)"
        );

        // Verify there is a ToolResult with success: false
        let has_failure = events.iter().any(|e| {
            matches!(
                e,
                SSEEvent::ToolResult {
                    name: _,
                    success: false
                }
            )
        });
        assert!(
            has_failure,
            "Debe haber un SSEEvent::ToolResult con success: false para el tool fallido"
        );

        // Verify Chunk events exist (LLM response after error)
        let chunk_events: Vec<&SSEEvent> = events
            .iter()
            .filter(|e| matches!(e, SSEEvent::Chunk { .. }))
            .collect();
        assert!(
            !chunk_events.is_empty(),
            "Debe emitirse SSEEvent::Chunk (el LLM responde incluso tras error del tool)"
        );

        // Verify Done event exists
        let done_events: Vec<&SSEEvent> = events
            .iter()
            .filter(|e| matches!(e, SSEEvent::Done { .. }))
            .collect();
        assert!(
            !done_events.is_empty(),
            "Debe emitirse SSEEvent::Done al completar el ReAct loop"
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Mock tool that counts calls — for the retry limit test
    // -----------------------------------------------------------------------

    struct MockLimitedTool {
        call_count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Tool for MockLimitedTool {
        fn name(&self) -> &'static str {
            "limited_tool"
        }

        fn description(&self) -> &'static str {
            "Tool that counts calls"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        fn permission(&self) -> Permission {
            Permission::NoConfirm
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult {
                success: true,
                data: serde_json::json!({"status": "ok"}),
                message: None,
            })
        }
    }

    // -----------------------------------------------------------------------
    // Mock LLM that calls limited_tool MAX_TOOL_RETRIES + 1 times,
    // then returns plain text on the next call.
    // -----------------------------------------------------------------------

    struct MockLLMExceedingRetries {
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl LLMProvider for MockLLMExceedingRetries {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            if *count <= MAX_TOOL_RETRIES + 1 {
                // Return a tool call for limited_tool
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Let me try...".into(),
                        tool_calls: Some(vec![ToolCall {
                            id: format!("call-{}", count),
                            name: "limited_tool".into(),
                            arguments: serde_json::json!({"input": count.to_string()}),
                        }]),
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            } else {
                // Final answer after exhausting retries
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Done after retries.".into(),
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
            request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            let result = self.chat(request).await?;
            let content = result.message.content.clone();
            let tool_calls = result.message.tool_calls.clone();

            let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();

            if let Some(tcs) = tool_calls {
                for tc in tcs {
                    events.push(Ok(StreamEvent::ToolCall(tc)));
                }
                events.push(Ok(StreamEvent::Done(result)));
            } else {
                for chunk in content
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(10)
                    .map(|c| c.iter().collect::<String>())
                {
                    events.push(Ok(StreamEvent::Chunk(chunk)));
                }
                events.push(Ok(StreamEvent::Done(result)));
            }

            let stream = futures::stream::iter(events);
            Ok(Box::pin(stream))
        }
    }

    // -----------------------------------------------------------------------
    // Test: tool should not execute more than 3 times in the same ReAct loop
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_tool_max_retries_in_react_loop() -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory SQLite pool and run migrations
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Registry with MockLimitedTool
        let call_count = Arc::new(AtomicUsize::new(0));
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockLimitedTool {
            call_count: call_count.clone(),
        }));
        let registry = Arc::new(registry);

        // 3. Mock LLM that calls limited_tool MAX_TOOL_RETRIES + 1 times
        let llm = Arc::new(MockLLMExceedingRetries {
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
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        let (tx, mut rx) = mpsc::channel(100);

        // 4. Execute
        let result = orchestrator
            .process_message_stream("profile-id", "test", None, tx)
            .await;

        // 5. Verify: orchestrator should complete successfully
        assert!(
            result.is_ok(),
            "Orchestrator should complete successfully, got: {:?}",
            result
        );

        // 6. Verify: tool should be called max MAX_TOOL_RETRIES times
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            MAX_TOOL_RETRIES,
            "Tool should be called max {} times, but was called {} times",
            MAX_TOOL_RETRIES,
            call_count.load(Ordering::SeqCst)
        );

        // 7. Verify SSE events: must have Done
        let mut got_done = false;
        while let Some(event) = rx.recv().await {
            if matches!(event, SSEEvent::Done { .. }) {
                got_done = true;
                break;
            }
        }
        assert!(got_done, "Should emit Done event");

        Ok(())
    }

    // -----------------------------------------------------------------------
    // T0.2: Orchestrator injects profile_id into tool calls
    // -----------------------------------------------------------------------

    /// Mock tool that records whether profile_id was injected.
    struct ProfileIdCaptureTool {
        profile_id_received: Arc<Mutex<bool>>,
    }

    #[async_trait::async_trait]
    impl Tool for ProfileIdCaptureTool {
        fn name(&self) -> &'static str {
            "capture_tool"
        }

        fn description(&self) -> &'static str {
            "Tool that captures profile_id"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        fn permission(&self) -> Permission {
            Permission::NoConfirm
        }

        async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, ToolError> {
            let has_profile_id =
                args.get("profile_id").and_then(|v| v.as_str()) == Some("profile-id");
            *self.profile_id_received.lock().unwrap() = has_profile_id;
            Ok(ToolResult {
                success: true,
                data: serde_json::json!({"ok": true}),
                message: None,
            })
        }
    }

    /// Mock LLM that returns a tool call on first invocation (without profile_id),
    /// then plain text on subsequent calls.
    struct MockLLMWithToolCallNoProfile {
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl LLMProvider for MockLLMWithToolCallNoProfile {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            if *count == 1 {
                // First call: return a tool call WITHOUT profile_id
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Let me process that.".into(),
                        tool_calls: Some(vec![ToolCall {
                            id: "call-1".into(),
                            name: "capture_tool".into(),
                            arguments: serde_json::json!({"some_arg": "value"}),
                        }]),
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                })
            } else {
                // Subsequent calls: return plain text answer
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "Done.".into(),
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
            request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            let result = self.chat(request).await?;
            let tool_calls = result.message.tool_calls.clone();
            let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();
            if let Some(tcs) = tool_calls {
                for tc in tcs {
                    events.push(Ok(StreamEvent::ToolCall(tc)));
                }
            }
            events.push(Ok(StreamEvent::Done(result)));
            let stream = futures::stream::iter(events);
            Ok(Box::pin(stream))
        }
    }

    #[tokio::test]
    async fn test_orchestrator_injects_profile_id_into_tool_call(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory SQLite pool
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Create registry with capture tool
        let profile_id_received = Arc::new(Mutex::new(false));
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ProfileIdCaptureTool {
            profile_id_received: profile_id_received.clone(),
        }));
        let registry = Arc::new(registry);

        // 3. Create mock LLM that returns tool call WITHOUT profile_id
        let call_count = Arc::new(Mutex::new(0));
        let llm = Arc::new(MockLLMWithToolCallNoProfile {
            call_count: call_count.clone(),
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
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        // 4. Call process_message (non-streaming) with profile-id
        let result = orchestrator
            .process_message("profile-id", "test message")
            .await;

        // 5. Verify: orchestrator should complete
        assert!(
            result.is_ok(),
            "Orchestrator should complete, got: {:?}",
            result
        );

        // 6. Verify: tool received profile_id injected by orchestrator
        assert!(
            *profile_id_received.lock().unwrap(),
            "Tool should have received profile_id='profile-id' injected by orchestrator"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_injects_profile_id_into_tool_call(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory SQLite pool
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Create registry with capture tool
        let profile_id_received = Arc::new(Mutex::new(false));
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ProfileIdCaptureTool {
            profile_id_received: profile_id_received.clone(),
        }));
        let registry = Arc::new(registry);

        // 3. Create mock LLM
        let call_count = Arc::new(Mutex::new(0));
        let llm = Arc::new(MockLLMWithToolCallNoProfile {
            call_count: call_count.clone(),
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
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        // 4. Call process_message_stream (streaming path)
        let (tx, _rx) = mpsc::channel(100);
        let result = orchestrator
            .process_message_stream("profile-id", "gestiona mi agenda", None, tx)
            .await;

        // 5. Verify: streaming completes successfully
        assert!(
            result.is_ok(),
            "Streaming should complete, got: {:?}",
            result
        );

        // 6. Verify: tool received profile_id
        assert!(
            *profile_id_received.lock().unwrap(),
            "Streaming path should inject profile_id into tool calls"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_calendar_description_includes_agenda_keywords(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Verify that the calendar tool description helps LLM route "agenda" correctly
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let tool = crate::tools::calendar::CalendarTool::new(pool);
        let desc = tool.description();
        assert!(
            desc.to_lowercase().contains("agenda"),
            "Calendar description should contain 'agenda', got: {}",
            desc
        );
        assert!(
            desc.contains("eventos") || desc.contains("citas"),
            "Calendar description should contain 'eventos' or 'citas', got: {}",
            desc
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_reminders_description_includes_alarm_keywords(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let tool = crate::tools::reminders::RemindersTool::new(pool);
        let desc = tool.description();
        assert!(
            desc.contains("alarmas") || desc.contains("avisos"),
            "Reminders description should contain 'alarmas' or 'avisos', got: {}",
            desc
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Mock LLM that captures ChatRequest to inspect system prompt
    // -----------------------------------------------------------------------

    struct SystemPromptCaptureLLM {
        captured: Arc<Mutex<Option<String>>>,
    }

    #[async_trait::async_trait]
    impl LLMProvider for SystemPromptCaptureLLM {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
            if let Some(msg) = request.messages.first() {
                *self.captured.lock().unwrap() = Some(msg.content.clone());
            }
            Ok(ChatResponse {
                message: ChatMessage {
                    role: "assistant".into(),
                    content: "OK.".into(),
                    tool_calls: None,
                    tool_result: None,
                    tool_call_id: None,
                },
                usage: None,
            })
        }

        async fn chat_stream(
            &self,
            request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            if let Some(msg) = request.messages.first() {
                *self.captured.lock().unwrap() = Some(msg.content.clone());
            }
            let events: Vec<Result<StreamEvent, LLMError>> =
                vec![Ok(StreamEvent::Done(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: "OK.".into(),
                        tool_calls: None,
                        tool_result: None,
                        tool_call_id: None,
                    },
                    usage: None,
                }))];
            let stream = futures::stream::iter(events);
            Ok(Box::pin(stream))
        }
    }

    #[tokio::test]
    async fn test_custom_system_prompt_from_db_is_used_in_stream(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Create in-memory DB with migrations
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // 2. Seed default settings then OVERRIDE system_prompt with a custom value
        crate::db::repos::settings::SettingsRepo::seed_defaults(&pool).await?;
        crate::db::repos::settings::SettingsRepo::set(
            &pool,
            "system_prompt",
            "Eres un asistente de pruebas. Responde siempre en español.",
        )
        .await?;
        crate::db::repos::settings::SettingsRepo::set(&pool, "max_window_tokens", "10000").await?;

        // 3. Create mock LLM that captures the system prompt
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let llm = Arc::new(SystemPromptCaptureLLM {
            captured: captured.clone(),
        });

        // 4. Build orchestrator components
        let registry = Arc::new(crate::tools::registry::ToolRegistry::new());
        let guardrails = Arc::new(crate::orchestrator::guardrails::Guardrails::new(
            registry.clone(),
        ));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config.clone(),
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        // 5. Call process_message_stream
        let (tx, mut rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("test-profile", "Hola", None, tx)
            .await?;

        // Drain rx to ensure processing completed
        while rx.recv().await.is_some() {}

        // 6. Assert the custom system prompt was used
        let captured_prompt = captured.lock().unwrap().clone();
        assert_eq!(
            captured_prompt.as_deref(),
            Some("Eres un asistente de pruebas. Responde siempre en español."),
            "System prompt should be the custom value from DB, not the default template. Got: {:?}",
            captured_prompt
        );

        // Also verify it is NOT the default template
        if let Some(ref p) = captured_prompt {
            assert!(
                !p.contains("mayordomo británico"),
                "System prompt should NOT be the default template, got: {}",
                p
            );
        }

        Ok(())
    }

    #[test]
    fn test_format_browser_timestamp_with_timezone() {
        // 06:23 UTC on 2026-09-26 = 08:23 CEST (Europe/Madrid, UTC+2)
        let result = format_browser_timestamp("2026-09-26T06:23:55.149Z", "Europe/Madrid");
        assert!(result.is_some());
        let s = result.unwrap();
        // Should say "son las 8:23 de la mañana" (local time), NOT "6:23"
        assert!(s.contains("8:23"), "Expected local time 8:23, got: {}", s);
        assert!(s.contains("de la mañana"), "Expected morning, got: {}", s);
        assert!(s.contains("sábado"), "Expected sábado");
        assert!(s.contains("26 de septiembre de 2026"));
    }

    #[test]
    fn test_format_browser_timestamp_utc() {
        // UTC timestamp with UTC timezone should show UTC time
        let result = format_browser_timestamp("2026-09-26T06:23:55.149Z", "UTC");
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.contains("6:23"), "Expected UTC time 6:23, got: {}", s);
    }

    // -----------------------------------------------------------------------
    // Stats recording tests (RED — orchestrator does NOT call record_request yet)
    // -----------------------------------------------------------------------

    /// Helper: create in-memory SQLite pool, run migrations, seed a default profile.
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("failed to create in-memory pool");

        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();

        // Seed a default profile for FK references
        sqlx::query(
            "INSERT OR IGNORE INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    /// Mock LLM that returns plain text immediately (no tool calls).
    struct SimpleTextLLM;

    #[async_trait::async_trait]
    impl LLMProvider for SimpleTextLLM {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
            Ok(ChatResponse {
                message: ChatMessage {
                    role: "assistant".into(),
                    content: "Simple response.".into(),
                    tool_calls: None,
                    tool_result: None,
                    tool_call_id: None,
                },
                usage: Some(TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    cached_tokens: 0,
                    reasoning_tokens: 0,
                    cost: 0.0,
                }),
            })
        }

        async fn chat_stream(
            &self,
            request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            let result = self.chat(request).await?;
            let content = result.message.content.clone();
            let mut events: Vec<Result<StreamEvent, LLMError>> = Vec::new();
            for chunk in content
                .chars()
                .collect::<Vec<_>>()
                .chunks(10)
                .map(|c| c.iter().collect::<String>())
            {
                events.push(Ok(StreamEvent::Chunk(chunk)));
            }
            events.push(Ok(StreamEvent::Done(result)));
            let stream = futures::stream::iter(events);
            Ok(Box::pin(stream))
        }
    }

    /// Mock LLM that always fails with an HTTP error.
    struct AlwaysFailingLLM;

    #[async_trait::async_trait]
    impl LLMProvider for AlwaysFailingLLM {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
            Err(LLMError::HttpError("fail".into()))
        }

        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>
        {
            Err(LLMError::HttpError("fail".into()))
        }
    }

    #[tokio::test]
    async fn test_process_message_records_stats() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;

        let llm = Arc::new(SimpleTextLLM);
        let registry = Arc::new(ToolRegistry::new());
        let guardrails = Arc::new(Guardrails::new(registry.clone()));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        let result = orchestrator.process_message("profile-1", "hello").await?;
        assert_eq!(result.iterations, 1);

        // RED: this assertion will fail because the orchestrator does not yet
        // call StatsRepo::record_request() after each LLM call.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM llm_requests")
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            count, 2,
            "Expected 2 rows in llm_requests (chat + reflection)."
        );

        // Both rows should have status 'success'
        let statuses: Vec<String> =
            sqlx::query_scalar("SELECT status FROM llm_requests ORDER BY created_at")
                .fetch_all(&pool)
                .await?;
        assert_eq!(statuses, vec!["success".to_string(), "success".to_string()]);

        Ok(())
    }

    #[tokio::test]
    async fn test_process_message_stream_records_stats() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;

        let llm = Arc::new(SimpleTextLLM);
        let registry = Arc::new(ToolRegistry::new());
        let guardrails = Arc::new(Guardrails::new(registry.clone()));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        let (tx, _rx) = mpsc::channel(100);
        orchestrator
            .process_message_stream("profile-1", "hello", None, tx)
            .await?;

        // RED: this assertion will fail because the orchestrator does not yet
        // call StatsRepo::record_request() after each LLM call.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM llm_requests")
            .fetch_one(&pool)
            .await?;
        assert!(
            count >= 1,
            "Expected at least 1 row in llm_requests. RED: record_request is not yet called."
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_process_message_records_stats_on_llm_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_test_db().await;

        let llm = Arc::new(AlwaysFailingLLM);
        let registry = Arc::new(ToolRegistry::new());
        let guardrails = Arc::new(Guardrails::new(registry.clone()));
        let context_builder = Arc::new(ContextBuilder::new());
        let config = OrchestratorConfig::default();

        let orchestrator = Orchestrator::new(
            llm,
            registry,
            guardrails,
            context_builder,
            config,
            pool.clone(),
            None,
            None,
            Arc::new(RwLock::new(None)),
        );

        let result = orchestrator.process_message("profile-1", "hello").await;
        assert!(
            result.is_err(),
            "process_message should return an error with AlwaysFailingLLM"
        );

        // RED: this assertion will fail because the orchestrator does not yet
        // call StatsRepo::record_request() even on error paths.
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM llm_requests WHERE status = 'error'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(
            count, 1,
            "Expected 1 row with status='error' in llm_requests. RED: record_request is not yet called on error."
        );

        Ok(())
    }
}

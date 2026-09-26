use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use chrono::Datelike;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::llm::provider::{ChatMessage, ChatRequest, LLMProvider, StreamEvent, ToolCall};
use crate::orchestrator::context_builder::ContextBuilder;
use crate::orchestrator::context_classifier::ContextClassifier;
use crate::orchestrator::guardrails::{GuardrailResult, Guardrails};
use crate::tools::r#trait::ToolResult;
use crate::tools::registry::ToolRegistry;
use futures::StreamExt;

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
                "Eres Alfred, un mayordomo británico al servicio del caballero. \
Seco, eficaz, con un punto de humor. Tratas al usuario de **usted**. \
Sirves la información sin adornos, a menos que te pidan que la sirvas con teatro.\n\n\
## Modo por defecto: conciso\n\
- Al grano. El dato primero, la floritura cuando toque.\n\
- No ofrezcas lo que no te han pedido. El caballero sabe lo que quiere.\n\
- El humor, una línea basta.\n\n\
## Modo expandido (solo a petición)\n\
Si el usuario dice \"explícame\", \"cuéntame\", \"dame más\" o similar, \
ahí sí puedes extenderte con libertad. \
Hasta entonces: señor, sí, señor.\n\n\
## Formato\n\
Usa Markdown completo: *cursivas*, **negritas**, listas, tablas, \
lo que el mensaje merezca. Usa emojis sin reparo."
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

/// Spanish day-of-week names (ISO weekday: 1 = Monday … 7 = Sunday).
const DIAS: [&str; 7] = [
    "lunes",
    "martes",
    "miércoles",
    "jueves",
    "viernes",
    "sábado",
    "domingo",
];

/// Spanish month names.
const MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

/// Spanish time-of-day phrases keyed by hour.
fn momento_del_dia(hora: u32) -> &'static str {
    match hora {
        0..=5 => "de la madrugada",
        6..=11 => "de la mañana",
        12..=20 => "de la tarde",
        _ => "de la noche",
    }
}

/// Parse an ISO‑8601 UTC timestamp and return a human‑readable Spanish string.
///
/// Returns `None` on parse failure so callers can fall back gracefully.
fn format_browser_timestamp(iso: &str, tz: &str) -> Option<String> {
    use chrono::NaiveDateTime;
    use chrono_tz::Tz;
    use std::str::FromStr;

    // Accept both "2026-09-24T08:00:00Z" and "2026-09-26T10:00:00.000Z"
    let naive =
        NaiveDateTime::parse_from_str(iso.trim_end_matches('Z'), "%Y-%m-%dT%H:%M:%S%.f").ok()?;

    let utc_dt: chrono::DateTime<chrono::Utc> =
        chrono::DateTime::from_naive_utc_and_offset(naive, chrono::Utc);

    // Convert to user's timezone
    let tz = Tz::from_str(tz).ok()?;
    let dt = utc_dt.with_timezone(&tz);

    let wd = dt.format("%u").to_string().parse::<usize>().ok()?; // 1–7
    let day_name = DIAS.get(wd - 1)?;
    let month_name = MESES.get((dt.month0()) as usize)?;
    let momento = momento_del_dia(dt.hour());

    Some(format!(
        "Hoy es {}, {} de {} de {}, son las {}:{:02} {}",
        day_name,
        dt.day(),
        month_name,
        dt.year(),
        dt.hour(),
        dt.minute(),
        momento,
    ))
}

/// Reverse‑geocode coordinates via Nominatim and return a short location name.
///
/// Results are cached in memory for 5 minutes, keyed by coordinates rounded
/// to 2 decimal places (≈1 km precision).
///
/// Returns `None` on any error (network, parse, no results) so callers can
/// fall back to showing raw coordinates.
async fn reverse_geocode(lat: f64, lon: f64) -> Option<String> {
    use std::time::Instant;

    /// Cache key: (lat×2000, lon×2000) rounded to integers (~55 m precision).
    type CacheKey = (i32, i32);
    /// Cache value: (when cached, location name).
    type CacheVal = (Instant, String);

    static CACHE: LazyLock<Mutex<HashMap<CacheKey, CacheVal>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let key: CacheKey = ((lat * 2000.0).round() as i32, (lon * 2000.0).round() as i32);

    // Check cache (5-minute TTL)
    {
        let cache = CACHE.lock().unwrap();
        if let Some((cached_at, name)) = cache.get(&key) {
            if cached_at.elapsed() < std::time::Duration::from_secs(300) {
                return Some(name.clone());
            }
        }
    }

    let url = format!(
        "https://nominatim.openstreetmap.org/reverse?lat={}&lon={}&format=json&addressdetails=1",
        lat, lon
    );

    let client = reqwest::Client::builder()
        .user_agent("Alfred/0.5 (alfred@atareao.es)")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;

    if body.get("error").is_some() {
        return None;
    }

    let addr = body.get("address")?;
    // Build a compact location string: street, town/city, region, country
    let mut parts: Vec<String> = Vec::new();

    // Street + house number
    if let Some(road) = addr.get("road").and_then(|v| v.as_str()) {
        let s = match addr.get("house_number").and_then(|v| v.as_str()) {
            Some(n) => format!("{} {}", road, n),
            None => road.to_string(),
        };
        parts.push(s);
    }

    // Town / city / village / hamlet (first match wins, dedup against street)
    for key in ["town", "city", "village", "hamlet"] {
        if let Some(v) = addr.get(key).and_then(|v| v.as_str()) {
            if parts.last().map(|l| l != v).unwrap_or(true) {
                parts.push(v.to_string());
                break;
            }
        }
    }

    // State, country
    for key in ["state", "country"] {
        if let Some(v) = addr.get(key).and_then(|v| v.as_str()) {
            if parts.last().map(|l| l != v).unwrap_or(true) {
                parts.push(v.to_string());
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        // Deduplicate consecutive identical parts (e.g. same city and village)
        let mut dedup: Vec<&str> = Vec::new();
        for p in &parts {
            if dedup.last().map(|&l| l != p.as_str()).unwrap_or(true) {
                dedup.push(p.as_str());
            }
        }
        let result = dedup.join(", ");

        // Store in cache
        {
            let mut cache = CACHE.lock().unwrap();
            cache.insert(key, (Instant::now(), result.clone()));
        }

        Some(result)
    }
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
    pub db: SqlitePool,
    pub collapse_tx: Option<mpsc::Sender<String>>,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        registry: Arc<ToolRegistry>,
        guardrails: Arc<Guardrails>,
        context_builder: Arc<ContextBuilder>,
        config: OrchestratorConfig,
        db: SqlitePool,
        collapse_tx: Option<mpsc::Sender<String>>,
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
        }
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

        let custom_prompt =
            crate::db::repos::settings::SettingsRepo::get(&self.db, "system_prompt")
                .await?
                .filter(|s| !s.is_empty());

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
                model: "default".into(),
                messages: messages.clone(),
                tools: Some(self.registry.definitions()),
                temperature: None,
                max_tokens: Some(self.config.max_tokens_per_turn),
                stream: false,
            };

            let response = self.llm.chat(request).await?;

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
                            if *tool_count > 3 {
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
                let analyzer = ReflectionAnalyzer::new(self.llm.clone());
                Some(analyzer.analyze(&messages, &message).await?)
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

        let custom_prompt =
            crate::db::repos::settings::SettingsRepo::get(&self.db, "system_prompt")
                .await?
                .filter(|s| !s.is_empty());

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
            let fecha = format_browser_timestamp(&ctx.timestamp, &ctx.timezone)
                .unwrap_or_else(|| ctx.timestamp.clone());

            let mut parts = vec![format!("{}. Zona horaria: {}.", fecha, ctx.timezone)];

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
                    Some(name) => parts.push(format!(
                        "El usuario está en {} ({:.4}, {:.4}).",
                        name, lat, lon
                    )),
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
        let user_message_id = {
            let collapse_callback = self.collapse_tx.clone().map(|tx| {
                Box::new(move |msg_id: String| {
                    let _ = tx.try_send(msg_id);
                }) as Box<dyn Fn(String) + Send>
            });
            let msg = crate::db::repos::messages::MessagesRepo::create(
                &self.db,
                "user",
                user_message,
                None,
                None,
                2000,
                collapse_callback,
            )
            .await?;
            msg.id
        };

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
                model: "default".into(),
                messages: messages.clone(),
                tools: Some(self.registry.definitions()),
                temperature: None,
                max_tokens: Some(self.config.max_tokens_per_turn),
                stream: true,
            };

            let mut stream = self.llm.chat_stream(request).await?;

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
                            .or_else(|| tool_calls_from_stream.take());
                        if let Some(tcs) = tool_calls {
                            let content = content_buffer.clone();

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
                                        let op = tc.arguments.get("operation").and_then(|v| v.as_str());
                                        let op_key = match op {
                                            Some(op_val) => format!("{}::{}", tc.name, op_val),
                                            None => tc.name.clone(),
                                        };
                                        let tool_count =
                                            tool_call_counts.entry(op_key).or_insert(0);
                                        *tool_count += 1;
                                        if *tool_count > 3 {
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
                                                used_tools.push(tc.name.clone());

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
                            let final_text = if !used_tools.is_empty() {
                                let footer = format!("\n\n---\n🔧 {}", used_tools.join(" · "));
                                format!("{}{}", content_buffer, footer)
                            } else {
                                content_buffer.clone()
                            };

                            // Persist assistant message to DB (capture the real UUID)
                            let assistant_message_id = {
                                let collapse_callback = self.collapse_tx.clone().map(|tx| {
                                    Box::new(move |msg_id: String| {
                                        let _ = tx.try_send(msg_id);
                                    })
                                        as Box<dyn Fn(String) + Send>
                                });
                                let msg = crate::db::repos::messages::MessagesRepo::create(
                                    &self.db,
                                    "assistant",
                                    &final_text,
                                    None,
                                    None,
                                    2000,
                                    collapse_callback,
                                )
                                .await?;
                                msg.id
                            };

                            let _ = tx
                                .send(SSEEvent::Done {
                                    message_id: assistant_message_id,
                                    user_message_id: user_message_id.clone(),
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
        // New persona: mayordomo, conciso por defecto, expandido a petición
        assert!(
            config
                .system_prompt_template
                .contains("mayordomo británico"),
            "debe definirse como mayordomo"
        );
        assert!(
            config.system_prompt_template.contains("caballero"),
            "debe tratar al usuario de caballero"
        );
        assert!(
            config
                .system_prompt_template
                .contains("Modo por defecto: conciso"),
            "debe tener modo conciso por defecto"
        );
        assert!(
            config.system_prompt_template.contains("expandido"),
            "debe tener modo expandido"
        );
        assert!(
            config.system_prompt_template.contains("emojis")
                || config.system_prompt_template.contains("Emojis"),
            "debe permitir emojis"
        );
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

            async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
                Ok(vec![])
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

        async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
            Ok(vec![])
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
        );

        // 4. Call process_message_stream and capture SSE events
        let (tx, mut rx) = mpsc::channel(100);
        let result = orchestrator
            .process_message_stream("profile-id", "Look something up", None, tx)
            .await;

        // 5. Collect all SSE events with a timeout
        let mut events: Vec<SSEEvent> = Vec::new();
        loop {
            match tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await {
                Ok(Some(event)) => {
                    events.push(event);
                }
                Ok(None) | Err(_) => break,
            }
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
    // Mock tool that fails 3 times then succeeds — for the retry limit test
    // -----------------------------------------------------------------------

    struct MockThreeTimeTool {
        call_count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Tool for MockThreeTimeTool {
        fn name(&self) -> &'static str {
            "limited_tool"
        }

        fn description(&self) -> &'static str {
            "Tool that fails 3 times then succeeds"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        fn permission(&self) -> Permission {
            Permission::NoConfirm
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, ToolError> {
            let count = self.call_count.fetch_add(1, Ordering::SeqCst);
            if count < 3 {
                Err(ToolError::ExecutionError(format!(
                    "Attempt {} failed",
                    count + 1
                )))
            } else {
                Ok(ToolResult {
                    success: true,
                    data: serde_json::json!({"status": "ok"}),
                    message: None,
                })
            }
        }
    }

    // -----------------------------------------------------------------------
    // Mock LLM that returns a tool call for limited_tool 4 times,
    // then returns plain text on the 5th call.
    // -----------------------------------------------------------------------

    struct MockLLM4Times {
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl LLMProvider for MockLLM4Times {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            if *count <= 4 {
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

        async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
            Ok(vec![])
        }
    }

    // -----------------------------------------------------------------------
    // Test: tool should not execute more than 3 times in the same ReAct loop
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_tool_max_3_retries_in_react_loop() -> Result<(), Box<dyn std::error::Error>> {
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

        // 2. Registry with MockThreeTimeTool
        let call_count = Arc::new(AtomicUsize::new(0));
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockThreeTimeTool {
            call_count: call_count.clone(),
        }));
        let registry = Arc::new(registry);

        // 3. Mock LLM that calls limited_tool 4 times
        let llm = Arc::new(MockLLM4Times {
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

        // 6. Verify: tool should be called max 3 times (4th call is blocked)
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            3,
            "Tool should be called max 3 times, but was called {} times",
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

        async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
            Ok(vec![])
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

        async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
            Ok(vec![])
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
}

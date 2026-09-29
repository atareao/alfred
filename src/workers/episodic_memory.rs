use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use sqlx::Row;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing;
use uuid::Uuid;

use crate::db::repos::memory::MemoryRepo;
use crate::db::repos::stats::StatsRepo;
use crate::llm::provider::{ChatMessage, ChatRequest, LLMProvider};
use crate::models::message::estimate_markdown_tokens_heuristic;

/// A lightweight representation of a message for batch processing.
#[derive(Debug, Clone)]
struct UnindexedMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub tokens_count: usize,
    pub created_at: String,
}

/// Configuration for the [`EpisodicMemoryWorker`].
#[derive(Debug, Clone)]
pub struct EpisodicMemoryConfig {
    /// Maximum accumulated tokens in a batch before forcing processing
    /// (default: 2000).
    pub batch_tokens: usize,
    /// Minutes of inactivity after which a partial batch is flushed
    /// (default: 30).
    pub inactivity_minutes: i64,
    /// Number of messages to include before/after the batch for context
    /// (default: 2).
    pub overlap: i64,
    /// Polling interval in minutes for periodic evaluation
    /// (default: 30).
    pub poll_interval_minutes: u64,
    /// LLM model used for generating episodic memory cards
    /// (default: `"mistralai/mistral-small-24b-instruct-2501"`).
    pub model: String,
}

impl Default for EpisodicMemoryConfig {
    fn default() -> Self {
        Self {
            batch_tokens: 2000,
            inactivity_minutes: 30,
            overlap: 2,
            poll_interval_minutes: 30,
            model: "mistralai/mistral-small-24b-instruct-2501".into(),
        }
    }
}

/// Archivist prompt used to synthesise episodic memory cards from raw
/// message blocks.
const ARCHIVIST_PROMPT: &str = r#"System: Eres un archivista de memoria para un asistente personal.
Tu tarea es leer la siguiente conversación y redactar una FICHA DE MEMORIA concisa (entre 150 y 300 palabras).

Devuelve la ficha en este formato exacto:

- FECHA/CONTEXTO: [Fecha o tema general del bloque]
- TEMAS TRATADOS: [Lista de conceptos clave, tecnologías o archivos mencionados]
- HECHOS Y DECISIONES: [Qué se hizo, qué problemas se resolvieron, datos concretos (puertos, IPs, comandos, variables, nombres de archivos)]
- SÍNTESIS: [Un resumen narrativo corto de lo que pasó en esta interacción]

Conversación a procesar:
{{ BLOQUE_DE_MENSAJES }}"#;

/// Tracks the Unix timestamp (in seconds) of the last failed LLM call.
/// Used to avoid rapid retries when the LLM returns unparseable responses.
static LAST_LLM_ATTEMPT: AtomicI64 = AtomicI64::new(0);

/// A structured memory card extracted from an LLM response.
#[derive(Debug, Clone)]
struct MemoryCard {
    pub date_context: String,
    pub topics: String,
    pub facts: String,
    pub synthesis: String,
}

/// Background worker that groups unindexed messages into episodic memory
/// cards ("fichas") via an LLM.
///
/// The worker is triggered by two sources:
/// - A signal on the `memory_rx` channel (sent when new messages are created).
/// - A periodic timer (`poll_interval_minutes`).
pub struct EpisodicMemoryWorker;

impl EpisodicMemoryWorker {
    /// Start the episodic memory worker in a new tokio task.
    ///
    /// The worker loops forever (or until `shutdown_rx` fires), selecting
    /// on a channel signal, an interval tick, and a shutdown signal.
    /// On each trigger it calls [`evaluate`](Self::evaluate) to process any
    /// unindexed messages.
    pub fn start(
        db: SqlitePool,
        llm_provider: Arc<dyn LLMProvider>,
        mut memory_rx: mpsc::Receiver<()>,
        mut shutdown_rx: broadcast::Receiver<()>,
        config: EpisodicMemoryConfig,
    ) -> JoinHandle<()> {
        let poll_interval = std::time::Duration::from_secs(config.poll_interval_minutes * 60);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(poll_interval);
            // Tick immediately on start
            interval.tick().await;

            loop {
                tokio::select! {
                    _ = memory_rx.recv() => {
                        tracing::debug!("EpisodicMemoryWorker triggered by channel signal");
                        Self::evaluate(&db, llm_provider.clone(), &config).await;
                    }
                    _ = interval.tick() => {
                        tracing::debug!("EpisodicMemoryWorker triggered by timer");
                        Self::evaluate(&db, llm_provider.clone(), &config).await;
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("EpisodicMemoryWorker shutting down");
                        break;
                    }
                }
            }
        })
    }

    /// Core evaluation logic: query unindexed messages, check conditions,
    /// build a batch (with overlap), call the LLM, and persist the result.
    pub(crate) async fn evaluate(
        db: &SqlitePool,
        llm_provider: Arc<dyn LLMProvider>,
        config: &EpisodicMemoryConfig,
    ) {
        // 1. Query unindexed messages
        let unindexed = match Self::query_unindexed_messages(db, config.batch_tokens).await {
            Ok(msgs) => msgs,
            Err(e) => {
                tracing::error!(error = %e, "EpisodicMemoryWorker: failed to query unindexed messages");
                return;
            }
        };

        if unindexed.is_empty() {
            tracing::debug!("EpisodicMemoryWorker: no unindexed messages to process");
            return;
        }

        // 2. Check conditions
        let total_tokens: usize = unindexed.iter().map(|m| m.tokens_count).sum();
        let should_process = Self::should_process_batch(&unindexed, total_tokens, config).await;

        if !should_process {
            tracing::debug!(
                total_tokens = %total_tokens,
                batch_tokens = %config.batch_tokens,
                "EpisodicMemoryWorker: batch does not meet processing conditions"
            );
            return;
        }

        // 3. Build batch with overlap
        let (primary, overlap_before, overlap_after) =
            match Self::build_batch_with_overlap(db, &unindexed, config).await {
                Ok(batch) => batch,
                Err(e) => {
                    tracing::error!(error = %e, "EpisodicMemoryWorker: failed to build batch");
                    return;
                }
            };

        if primary.is_empty() {
            tracing::warn!("EpisodicMemoryWorker: primary batch is empty, skipping");
            return;
        }

        // 4. Build the message block for the LLM
        let message_block = Self::format_message_block(&primary, &overlap_before, &overlap_after);

        // 5. Rate-limit: skip LLM call if we just failed recently
        let now_ts = chrono::Utc::now().timestamp();
        let last_attempt = LAST_LLM_ATTEMPT.load(Ordering::Relaxed);
        let cooldown_secs: i64 = 60; // fixed 60s cooldown after a failed LLM call

        if now_ts - last_attempt < cooldown_secs && last_attempt > 0 {
            let unindexed_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE is_indexed = 0")
                    .fetch_one(db)
                    .await
                    .unwrap_or(0);

            // Also check that the same messages are still unindexed
            // (count hasn't changed since last attempt)
            if unindexed_count > 0 {
                tracing::warn!(
                    unindexed_count = %unindexed_count,
                    seconds_since_last_attempt = %(now_ts - last_attempt),
                    "EpisodicMemoryWorker: skipping LLM call (rate-limited after previous failure)"
                );
                return;
            }
        }

        // 5b. Call LLM
        let card = match Self::call_llm(db, "episodic", &llm_provider, config, &message_block).await
        {
            Some(card) => {
                // Reset the failure tracker on success
                LAST_LLM_ATTEMPT.store(0, Ordering::Relaxed);
                card
            }
            None => {
                LAST_LLM_ATTEMPT.store(chrono::Utc::now().timestamp(), Ordering::Relaxed);
                tracing::error!(
                    "EpisodicMemoryWorker: LLM call failed or returned unparseable response"
                );
                return;
            }
        };

        // 6. Persist memory + embedding + update messages
        if let Err(e) = Self::persist(db, &llm_provider, &card, &primary).await {
            tracing::error!(error = %e, "EpisodicMemoryWorker: failed to persist memory card");
        }
    }

    // ─── Private helpers ──────────────────────────────────────────────────

    /// Query all unindexed messages ordered by `created_at ASC`.
    /// Uses a conservative limit to avoid loading too many at once.
    async fn query_unindexed_messages(
        db: &SqlitePool,
        _batch_tokens: usize,
    ) -> Result<Vec<UnindexedMessage>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, role, content, tokens_count, created_at
             FROM messages
             WHERE is_indexed = 0
             ORDER BY created_at ASC
             LIMIT 500",
        )
        .fetch_all(db)
        .await?;

        let messages = rows
            .iter()
            .map(|r| UnindexedMessage {
                id: r.get(0),
                role: r.get(1),
                content: r.get(2),
                tokens_count: r.get::<i64, _>(3) as usize,
                created_at: r.get(4),
            })
            .collect();

        Ok(messages)
    }

    /// Determine whether the current batch should be processed.
    async fn should_process_batch(
        unindexed: &[UnindexedMessage],
        total_tokens: usize,
        config: &EpisodicMemoryConfig,
    ) -> bool {
        // Condition A: token budget met
        if total_tokens >= config.batch_tokens {
            return true;
        }

        // Condition B: inactivity timeout (oldest message exceeds threshold)
        if let Some(oldest) = unindexed.first() {
            let now = chrono::Utc::now();
            let oldest_time = match chrono::DateTime::parse_from_rfc3339(&oldest.created_at) {
                Ok(t) => t.with_timezone(&chrono::Utc),
                Err(_) => return false,
            };
            let elapsed_minutes = (now - oldest_time).num_minutes();
            if elapsed_minutes >= config.inactivity_minutes {
                return true;
            }
        }

        false
    }

    /// Build the primary batch (within token budget) plus overlap messages
    /// before and after.
    async fn build_batch_with_overlap(
        db: &SqlitePool,
        unindexed: &[UnindexedMessage],
        config: &EpisodicMemoryConfig,
    ) -> Result<
        (
            Vec<UnindexedMessage>,
            Vec<UnindexedMessage>,
            Vec<UnindexedMessage>,
        ),
        sqlx::Error,
    > {
        // Primary batch: messages within token budget
        let mut cumulative = 0usize;
        let mut primary_end = 0;
        for (i, msg) in unindexed.iter().enumerate() {
            cumulative += msg.tokens_count;
            if cumulative >= config.batch_tokens {
                primary_end = i + 1;
                break;
            }
        }
        if primary_end == 0 {
            primary_end = unindexed.len();
        }
        let primary = unindexed[..primary_end].to_vec();

        if primary.is_empty() {
            return Ok((vec![], vec![], vec![]));
        }

        let first_id = &primary[0].id;
        let last_id = &primary[primary.len() - 1].id;

        // Overlap BEFORE: messages with created_at < first primary's created_at
        let overlap_rows = sqlx::query(
            "SELECT id, role, content, tokens_count, created_at
             FROM messages
             WHERE created_at < (SELECT created_at FROM messages WHERE id = ?1)
             ORDER BY created_at DESC
             LIMIT ?2",
        )
        .bind(first_id)
        .bind(config.overlap)
        .fetch_all(db)
        .await?;

        let mut overlap_before: Vec<UnindexedMessage> = overlap_rows
            .iter()
            .map(|r| UnindexedMessage {
                id: r.get(0),
                role: r.get(1),
                content: r.get(2),
                tokens_count: r.get::<i64, _>(3) as usize,
                created_at: r.get(4),
            })
            .collect();
        // Reverse so they appear chronologically
        overlap_before.reverse();

        // Overlap AFTER: messages with created_at > last primary's created_at
        let overlap_rows = sqlx::query(
            "SELECT id, role, content, tokens_count, created_at
             FROM messages
             WHERE created_at > (SELECT created_at FROM messages WHERE id = ?1)
             ORDER BY created_at ASC
             LIMIT ?2",
        )
        .bind(last_id)
        .bind(config.overlap)
        .fetch_all(db)
        .await?;

        let overlap_after: Vec<UnindexedMessage> = overlap_rows
            .iter()
            .map(|r| UnindexedMessage {
                id: r.get(0),
                role: r.get(1),
                content: r.get(2),
                tokens_count: r.get::<i64, _>(3) as usize,
                created_at: r.get(4),
            })
            .collect();

        Ok((primary, overlap_before, overlap_after))
    }

    /// Format the message block for the LLM prompt, tagging overlap messages.
    fn format_message_block(
        primary: &[UnindexedMessage],
        overlap_before: &[UnindexedMessage],
        overlap_after: &[UnindexedMessage],
    ) -> String {
        let mut parts = Vec::new();

        if !overlap_before.is_empty() {
            parts.push("[INICIO DE CONTEXTO ANTERIOR (overlap)]".to_string());
            for msg in overlap_before {
                parts.push(format!("[{}]: {}", msg.role, msg.content));
            }
            parts.push("[FIN DE CONTEXTO ANTERIOR (overlap)]".to_string());
        }

        parts.push("[INICIO DE BLOQUE PRINCIPAL A ARCHIVAR]".to_string());
        for msg in primary {
            parts.push(format!("[{}]: {}", msg.role, msg.content));
        }
        parts.push("[FIN DE BLOQUE PRINCIPAL A ARCHIVAR]".to_string());

        if !overlap_after.is_empty() {
            parts.push("[INICIO DE CONTEXTO POSTERIOR (overlap)]".to_string());
            for msg in overlap_after {
                parts.push(format!("[{}]: {}", msg.role, msg.content));
            }
            parts.push("[FIN DE CONTEXTO POSTERIOR (overlap)]".to_string());
        }

        parts.join("\n")
    }

    /// Call the LLM with the archivist prompt and message block.
    /// Parses the structured response into a `MemoryCard`.
    async fn call_llm(
        db: &SqlitePool,
        profile_id: &str,
        llm_provider: &Arc<dyn LLMProvider>,
        config: &EpisodicMemoryConfig,
        message_block: &str,
    ) -> Option<MemoryCard> {
        let system_content = ARCHIVIST_PROMPT.replace("{{ BLOQUE_DE_MENSAJES }}", message_block);

        tracing::debug!(
            prompt_preview = %system_content.chars().take(200).collect::<String>(),
            prompt_len = %system_content.len(),
            model = %config.model,
            "EpisodicMemoryWorker: LLM request prompt preview"
        );

        let request = ChatRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage {
                role: "system".into(),
                content: system_content,
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            }],
            tools: None,
            temperature: Some(0.3),
            max_tokens: Some(1024),
            stream: false,
        };

        let start = std::time::Instant::now();
        let response = match llm_provider.chat(request).await {
            Ok(r) => r,
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as i64;
                let _ = StatsRepo::record_request(
                    db,
                    &Uuid::new_v4().to_string(),
                    &config.model,
                    profile_id,
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
                tracing::error!(error = %e, "EpisodicMemoryWorker: LLM chat failed");
                return None;
            }
        };

        // Record stats for this LLM call
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
            db,
            &Uuid::new_v4().to_string(),
            &config.model,
            profile_id,
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

        let content = &response.message.content;
        let content_preview = content.chars().take(300).collect::<String>();
        let card = Self::parse_memory_card(content);

        if card.is_none() {
            tracing::debug!(
                content_len = %content.len(),
                content_preview = %content_preview,
                "EpisodicMemoryWorker: LLM response content preview (unparseable)"
            );
        }

        card
    }

    /// Parse the structured LLM response into a `MemoryCard`.
    ///
    /// Expected format:
    /// ```text
    /// - FECHA/CONTEXTO: ...
    /// - TEMAS TRATADOS: ...
    /// - HECHOS Y DECISIONES: ...
    /// - SÍNTESIS: ...
    /// ```
    fn parse_memory_card(response: &str) -> Option<MemoryCard> {
        let mut date_context = String::new();
        let mut topics = String::new();
        let mut facts = String::new();
        let mut synthesis = String::new();

        let mut current_section: Option<&mut String> = None;

        for line in response.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(rest) = trimmed
                .strip_prefix("- FECHA/CONTEXTO:")
                .or_else(|| trimmed.strip_prefix("- FECHA/CONTEXTO :"))
                .or_else(|| trimmed.strip_prefix("FECHA/CONTEXTO:"))
                .or_else(|| trimmed.strip_prefix("FECHA/CONTEXTO :"))
            {
                date_context = rest.trim().to_string();
                current_section = Some(&mut date_context);
                continue;
            }

            if let Some(rest) = trimmed
                .strip_prefix("- TEMAS TRATADOS:")
                .or_else(|| trimmed.strip_prefix("- TEMAS TRATADOS :"))
                .or_else(|| trimmed.strip_prefix("TEMAS TRATADOS:"))
                .or_else(|| trimmed.strip_prefix("TEMAS TRATADOS :"))
            {
                topics = rest.trim().to_string();
                current_section = Some(&mut topics);
                continue;
            }

            if let Some(rest) = trimmed
                .strip_prefix("- HECHOS Y DECISIONES:")
                .or_else(|| trimmed.strip_prefix("- HECHOS Y DECISIONES :"))
                .or_else(|| trimmed.strip_prefix("HECHOS Y DECISIONES:"))
                .or_else(|| trimmed.strip_prefix("HECHOS Y DECISIONES :"))
            {
                facts = rest.trim().to_string();
                current_section = Some(&mut facts);
                continue;
            }

            if let Some(rest) = trimmed
                .strip_prefix("- SÍNTESIS:")
                .or_else(|| trimmed.strip_prefix("- SÍNTESIS :"))
                .or_else(|| trimmed.strip_prefix("SÍNTESIS:"))
                .or_else(|| trimmed.strip_prefix("SÍNTESIS :"))
            {
                synthesis = rest.trim().to_string();
                current_section = Some(&mut synthesis);
                continue;
            }

            // Continuation line for the current section
            if let Some(ref mut section) = current_section {
                if !section.is_empty() {
                    section.push(' ');
                    section.push_str(trimmed);
                }
            }
        }

        // We need at least one meaningful section
        if date_context.is_empty() && topics.is_empty() && facts.is_empty() && synthesis.is_empty()
        {
            return None;
        }

        Some(MemoryCard {
            date_context,
            topics,
            facts,
            synthesis,
        })
    }

    /// Persist the memory card: store in `memory`, generate embedding,
    /// store in `vec_memory`, and update the indexed messages.
    async fn persist(
        db: &SqlitePool,
        llm_provider: &Arc<dyn LLMProvider>,
        card: &MemoryCard,
        primary: &[UnindexedMessage],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Build the canonical ficha text
        let ficha = format!(
            "- FECHA/CONTEXTO: {}\n- TEMAS TRATADOS: {}\n- HECHOS Y DECISIONES: {}\n- SÍNTESIS: {}",
            card.date_context, card.topics, card.facts, card.synthesis,
        );

        let tokens_count = estimate_markdown_tokens_heuristic(&ficha);

        // Metadata: include the primary message IDs as reference
        let primary_ids: Vec<&str> = primary.iter().map(|m| m.id.as_str()).collect();
        let metadata = serde_json::json!({
            "source": "episodic_worker",
            "primary_message_ids": primary_ids,
            "date_context": card.date_context,
        });

        // 1. Insert into `memory` table
        let memory = MemoryRepo::create(db, &ficha, tokens_count, &metadata).await?;

        // 2. Generate embedding via LLM provider
        let embedding = match llm_provider.embed(&ficha).await {
            Ok(emb) => emb,
            Err(e) => {
                return Err(format!("embedding generation failed: {}", e).into());
            }
        };

        // 3. Store embedding in `vec_memory`
        let embedding_json = serde_json::to_string(&embedding)?;
        sqlx::query("INSERT INTO vec_memory (id, embedding) VALUES (?1, ?2)")
            .bind(&memory.id)
            .bind(&embedding_json)
            .execute(db)
            .await?;

        // 4. Update primary messages: set is_indexed = 1, summary_ref = memory.id
        for msg in primary {
            sqlx::query("UPDATE messages SET is_indexed = 1, summary_ref = ?1 WHERE id = ?2")
                .bind(&memory.id)
                .bind(&msg.id)
                .execute(db)
                .await?;
        }

        tracing::info!(
            memory_id = %memory.id,
            primary_count = %primary.len(),
            tokens = %tokens_count,
            "EpisodicMemoryWorker: persisted memory card"
        );

        Ok(())
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Tests (RED → GREEN)
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use crate::llm::provider::{ChatMessage, ChatRequest, ChatResponse, LLMError, TokenUsage};
    use async_trait::async_trait;
    use serial_test::serial;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::sync::{Arc, Mutex};

    // ─── Mock LLM Provider ────────────────────────────────────────────────

    struct MockEpisodicLLM {
        pub chat_calls: Arc<Mutex<Vec<ChatRequest>>>,
        pub embed_calls: Arc<Mutex<Vec<String>>>,
        pub chat_response: String,
        pub embed_response: Vec<f32>,
    }

    #[async_trait]
    impl LLMProvider for MockEpisodicLLM {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
            self.chat_calls.lock().unwrap().push(request);
            Ok(ChatResponse {
                message: ChatMessage {
                    role: "assistant".into(),
                    content: self.chat_response.clone(),
                    tool_calls: None,
                    tool_result: None,
                    tool_call_id: None,
                },
                usage: Some(TokenUsage {
                    prompt_tokens: 500,
                    completion_tokens: 200,
                    cached_tokens: 0,
                    reasoning_tokens: 0,
                    cost: 0.0,
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

        async fn embed(&self, input: &str) -> Result<Vec<f32>, LLMError> {
            self.embed_calls.lock().unwrap().push(input.to_string());
            Ok(self.embed_response.clone())
        }
    }

    impl MockEpisodicLLM {
        fn new(chat_response: &str) -> Self {
            Self {
                chat_calls: Arc::new(Mutex::new(Vec::new())),
                embed_calls: Arc::new(Mutex::new(Vec::new())),
                chat_response: chat_response.to_string(),
                embed_response: vec![0.1, 0.2, 0.3],
            }
        }

        fn wrap(self) -> Arc<dyn LLMProvider> {
            Arc::new(self)
        }
    }

    // ─── Test helpers ─────────────────────────────────────────────────────

    async fn test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("Failed to create in-memory database");
        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        pool
    }

    /// Insert a message directly for testing, with full control over fields.
    async fn insert_message(
        pool: &SqlitePool,
        role: &str,
        content: &str,
        tokens_count: usize,
        is_indexed: bool,
        created_at: &str,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO messages (id, role, content, tokens_count, is_indexed, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(role)
        .bind(content)
        .bind(tokens_count as i64)
        .bind(is_indexed)
        .bind(created_at)
        .execute(pool)
        .await
        .expect("Failed to insert test message");
        id
    }

    async fn count_unindexed(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE is_indexed = 0")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn count_memory(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM memory")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn count_vec_memory(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM vec_memory")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    const SAMPLE_LLM_RESPONSE: &str = "\
- FECHA/CONTEXTO: 26 de septiembre de 2026 — Configuración de infraestructura
- TEMAS TRATADOS: Podman, Docker Compose, PostgreSQL, configuración de red, variables de entorno
- HECHOS Y DECISIONES: Se configuró Podman con docker-compose.yml. Se expuso el puerto 5432 para PostgreSQL. Se decidió usar la red `valet_net` con driver bridge. Se estableció la variable `POSTGRES_DB=valet`.
- SÍNTESIS: El equipo configuró el entorno de desarrollo con Podman, definiendo los servicios de base de datos y aplicación en un docker-compose.yml. Se resolvieron problemas de conexión entre contenedores ajustando las redes virtuales.";

    // ─── 3.1 / 3.2: Worker loop ───────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_worker_loop_receives_channel_signal() {
        // Reset the global rate limiter from any previous test
        LAST_LLM_ATTEMPT.store(0, Ordering::Relaxed);

        let db = test_db().await;
        let db = test_db().await;
        let (memory_tx, memory_rx) = mpsc::channel::<()>(16);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        // Insert some unindexed messages so evaluate has work to do
        let now = chrono::Utc::now().to_rfc3339();
        for i in 0..5 {
            insert_message(&db, "user", &format!("Message {}", i), 500, false, &now).await;
        }

        let mock = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE);
        let chat_calls = mock.chat_calls.clone();
        let provider = mock.wrap();

        let _handle = EpisodicMemoryWorker::start(
            db.clone(),
            provider,
            memory_rx,
            shutdown_rx,
            EpisodicMemoryConfig {
                poll_interval_minutes: 999, // long interval to avoid timer trigger
                ..Default::default()
            },
        );

        // Send signal
        memory_tx.send(()).await.unwrap();

        // Give the worker time to process
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // The LLM must have been called at least once
        let calls = chat_calls.lock().unwrap();
        assert!(
            !calls.is_empty(),
            "LLM provider chat() should have been called after channel signal"
        );

        // Clean up
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    #[serial]
    async fn test_worker_loop_shutdown() {
        let db = test_db().await;
        let (_, memory_rx) = mpsc::channel::<()>(16);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let mock = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE).wrap();

        let handle = EpisodicMemoryWorker::start(
            db.clone(),
            mock,
            memory_rx,
            shutdown_rx,
            EpisodicMemoryConfig::default(),
        );

        // Send shutdown
        shutdown_tx.send(()).unwrap();

        // The handle should complete (join) within a reasonable time
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;

        assert!(
            result.is_ok(),
            "Worker handle should complete after shutdown signal"
        );
    }

    // ─── 3.3 / 3.4: Query unindexed messages ─────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_evaluate_selects_unindexed_messages() {
        let db = test_db().await;
        let now = chrono::Utc::now().to_rfc3339();

        // 5 unindexed messages of 500 tokens each (2500 total > 2000 batch)
        for i in 0..5 {
            insert_message(
                &db,
                "user",
                &format!("Message content {}", i),
                500,
                false,
                &now,
            )
            .await;
        }

        let mock = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE);
        let chat_calls = mock.chat_calls.clone();
        let provider = mock.wrap();

        EpisodicMemoryWorker::evaluate(&db, provider, &EpisodicMemoryConfig::default()).await;

        let calls = chat_calls.lock().unwrap();
        assert!(
            !calls.is_empty(),
            "LLM should have been called when unindexed messages exist"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_evaluate_no_unindexed_messages() {
        let db = test_db().await;
        let now = chrono::Utc::now().to_rfc3339();

        // All indexed
        for i in 0..3 {
            insert_message(&db, "user", &format!("Message {}", i), 100, true, &now).await;
        }

        let mock = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE);
        let chat_calls = mock.chat_calls.clone();
        let provider = mock.wrap();

        EpisodicMemoryWorker::evaluate(&db, provider, &EpisodicMemoryConfig::default()).await;

        let calls = chat_calls.lock().unwrap();
        assert!(
            calls.is_empty(),
            "LLM should NOT be called when all messages are indexed"
        );
    }

    // ─── Batch size threshold ────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_evaluate_batch_size_threshold() {
        let db = test_db().await;
        let now = chrono::Utc::now().to_rfc3339();

        // 7 messages of 300 tokens each = 2100 >= 2000 batch
        for i in 0..7 {
            insert_message(&db, "user", &format!("Message {}", i), 300, false, &now).await;
        }

        let provider = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE).wrap();

        EpisodicMemoryWorker::evaluate(&db, provider, &EpisodicMemoryConfig::default()).await;

        assert_eq!(count_memory(&db).await, 1, "Should create one memory card");
    }

    #[tokio::test]
    #[serial]
    async fn test_evaluate_below_batch_threshold_and_recent() {
        let db = test_db().await;
        let now = chrono::Utc::now().to_rfc3339();

        // 3 messages of 200 tokens each = 600 < 2000 batch, and recent
        for i in 0..3 {
            insert_message(
                &db,
                "user",
                &format!("Recent short msg {}", i),
                200,
                false,
                &now,
            )
            .await;
        }

        let provider = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE).wrap();

        EpisodicMemoryWorker::evaluate(
            &db,
            provider,
            &EpisodicMemoryConfig {
                batch_tokens: 2000,
                inactivity_minutes: 30,
                ..Default::default()
            },
        )
        .await;

        assert_eq!(
            count_memory(&db).await,
            0,
            "Should NOT create memory when batch is below threshold and recent"
        );
    }

    // ─── Inactivity timeout ──────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_evaluate_inactivity_triggers_batch() {
        let db = test_db().await;
        let old_time = (chrono::Utc::now() - chrono::Duration::minutes(45)).to_rfc3339();

        // 3 messages of 200 tokens each (600 < 2000 batch), but old
        for i in 0..3 {
            insert_message(
                &db,
                "user",
                &format!("Old message {}", i),
                200,
                false,
                &old_time,
            )
            .await;
        }

        let provider = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE).wrap();

        EpisodicMemoryWorker::evaluate(
            &db,
            provider,
            &EpisodicMemoryConfig {
                batch_tokens: 2000,
                inactivity_minutes: 30,
                ..Default::default()
            },
        )
        .await;

        assert_eq!(
            count_memory(&db).await,
            1,
            "Should create memory card via inactivity trigger"
        );
    }

    // ─── 3.5 / 3.6: Batch with overlap ───────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_evaluate_batch_with_overlap() {
        let db = test_db().await;
        let base_time = chrono::Utc::now() - chrono::Duration::hours(1);

        // Overlap BEFORE (already indexed)
        insert_message(
            &db,
            "user",
            "Overlap before 1",
            100,
            true,
            &(base_time - chrono::Duration::minutes(10)).to_rfc3339(),
        )
        .await;
        insert_message(
            &db,
            "assistant",
            "Overlap before 2",
            100,
            true,
            &(base_time - chrono::Duration::minutes(9)).to_rfc3339(),
        )
        .await;

        // 7 PRIMARY messages (unindexed, 300 tokens each = 2100 total)
        for i in 0..7 {
            insert_message(
                &db,
                if i % 2 == 0 { "user" } else { "assistant" },
                &format!("Primary message {}", i),
                300,
                false,
                &(base_time + chrono::Duration::minutes(i)).to_rfc3339(),
            )
            .await;
        }

        // Overlap AFTER
        insert_message(
            &db,
            "user",
            "Overlap after 1",
            100,
            false,
            &(base_time + chrono::Duration::minutes(10)).to_rfc3339(),
        )
        .await;
        insert_message(
            &db,
            "assistant",
            "Overlap after 2",
            100,
            false,
            &(base_time + chrono::Duration::minutes(11)).to_rfc3339(),
        )
        .await;

        let mock = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE);
        let chat_calls = mock.chat_calls.clone();
        let provider = mock.wrap();

        EpisodicMemoryWorker::evaluate(
            &db,
            provider,
            &EpisodicMemoryConfig {
                batch_tokens: 2000,
                overlap: 2,
                ..Default::default()
            },
        )
        .await;

        let calls = chat_calls.lock().unwrap();
        assert!(!calls.is_empty(), "LLM should have been called");

        // The system message must contain overlap markers
        let request = &calls[0];
        let system_msg = request
            .messages
            .iter()
            .find(|m| m.role == "system")
            .expect("Should have a system message");

        assert!(
            system_msg.content.contains("CONTEXTO ANTERIOR (overlap)"),
            "Should include overlap BEFORE markers"
        );
        assert!(
            system_msg.content.contains("CONTEXTO POSTERIOR (overlap)"),
            "Should include overlap AFTER markers"
        );
        assert!(
            system_msg.content.contains("BLOQUE PRINCIPAL A ARCHIVAR"),
            "Should include primary block markers"
        );

        // Verify specific overlap messages are in the block
        assert!(
            system_msg.content.contains("Overlap before 1"),
            "Should contain overlap before message"
        );
        assert!(
            system_msg.content.contains("Overlap after 1"),
            "Should contain overlap after message"
        );
    }

    // ─── 3.7 / 3.8: LLM generates ficha ───────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_evaluate_parses_llm_response() {
        let db = test_db().await;
        let now = chrono::Utc::now().to_rfc3339();

        for i in 0..5 {
            insert_message(&db, "user", &format!("Message {}", i), 500, false, &now).await;
        }

        let mock = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE);
        let chat_calls = mock.chat_calls.clone();
        let provider = mock.wrap();

        EpisodicMemoryWorker::evaluate(&db, provider, &EpisodicMemoryConfig::default()).await;

        let calls = chat_calls.lock().unwrap();
        assert!(!calls.is_empty(), "LLM should have been called");

        // Memory card should have been created
        assert_eq!(count_memory(&db).await, 1, "One memory card should exist");

        // The content must contain parsed sections
        let (memories, _total) = MemoryRepo::list(&db, 10, 0).await.unwrap();
        assert_eq!(memories.len(), 1);
        let content = &memories[0].content;
        assert!(content.contains("Podman"), "Should contain parsed topic");
        assert!(
            content.contains("FECHA/CONTEXTO"),
            "Should contain date context section"
        );
        assert!(
            content.contains("SÍNTESIS"),
            "Should contain synthesis section"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_evaluate_unparseable_response_does_not_create_memory() {
        let db = test_db().await;
        let now = chrono::Utc::now().to_rfc3339();

        for i in 0..5 {
            insert_message(&db, "user", &format!("Message {}", i), 500, false, &now).await;
        }

        let provider = MockEpisodicLLM::new("Esto no tiene el formato esperado.").wrap();

        EpisodicMemoryWorker::evaluate(&db, provider, &EpisodicMemoryConfig::default()).await;

        assert_eq!(
            count_memory(&db).await,
            0,
            "No memory should be created for unparseable response"
        );
    }

    // ─── 3.9 / 3.10: Persistencia completa ───────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_evaluate_persists_memory_embedding_and_updates() {
        let db = test_db().await;
        let now = chrono::Utc::now().to_rfc3339();

        // 7 unindexed messages of 300 tokens each (2100 > 2000 batch)
        let mut msg_ids = Vec::new();
        for i in 0..7 {
            let id = insert_message(&db, "user", &format!("Message {}", i), 300, false, &now).await;
            msg_ids.push(id);
        }

        // Also insert an overlap BEFORE message (indexed) — should remain unchanged
        let overlap_id = insert_message(
            &db,
            "assistant",
            "Earlier context message",
            100,
            true,
            &(chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339(),
        )
        .await;

        let mock = MockEpisodicLLM::new(SAMPLE_LLM_RESPONSE);
        let chat_calls = mock.chat_calls.clone();
        let embed_calls = mock.embed_calls.clone();
        let provider = mock.wrap();

        EpisodicMemoryWorker::evaluate(&db, provider, &EpisodicMemoryConfig::default()).await;

        // 1. LLM chat was called
        assert!(
            !chat_calls.lock().unwrap().is_empty(),
            "LLM chat should have been called"
        );

        // 2. Embedding was generated
        let embed_calls = embed_calls.lock().unwrap();
        assert!(!embed_calls.is_empty(), "LLM embed should have been called");
        assert!(
            embed_calls[0].contains("FECHA/CONTEXTO"),
            "Embedding input should be the ficha text"
        );

        // 3. Memory row exists
        assert_eq!(count_memory(&db).await, 1, "Should have one memory row");

        // 4. Vec_memory row exists
        assert_eq!(
            count_vec_memory(&db).await,
            1,
            "Should have one vec_memory row"
        );

        // Verify embedding content
        let emb_row: String = sqlx::query_scalar("SELECT embedding FROM vec_memory LIMIT 1")
            .fetch_one(&db)
            .await
            .unwrap();
        let emb: Vec<f64> = serde_json::from_str(&emb_row).unwrap();
        assert!(!emb.is_empty(), "Embedding vector should not be empty");
        assert!(
            (emb[0] - 0.1).abs() < 0.01,
            "First embedding component should be 0.1"
        );

        // 5. All 7 primary messages are now indexed
        assert_eq!(
            count_unindexed(&db).await,
            0,
            "All messages should be indexed"
        );

        // Verify each primary message has is_indexed = 1 and summary_ref set
        for msg_id in &msg_ids {
            let row = sqlx::query("SELECT is_indexed, summary_ref FROM messages WHERE id = ?1")
                .bind(msg_id)
                .fetch_one(&db)
                .await
                .expect("Message should exist");

            let is_indexed: bool = row.get(0);
            let summary_ref: Option<String> = row.get(1);

            assert!(is_indexed, "Message {} should be indexed", msg_id);
            assert!(
                summary_ref.is_some(),
                "Message {} should have summary_ref set",
                msg_id
            );
        }

        // 6. Overlap message (already indexed) must NOT have its is_indexed state changed
        let overlap_row = sqlx::query("SELECT is_indexed, summary_ref FROM messages WHERE id = ?1")
            .bind(&overlap_id)
            .fetch_one(&db)
            .await
            .expect("Overlap message should exist");

        let overlap_indexed: bool = overlap_row.get(0);
        assert!(
            overlap_indexed,
            "Overlap message should still be indexed (was indexed before)"
        );
    }

    // ─── Additional: parse_memory_card unit tests ────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_parse_memory_card_full() {
        let card = EpisodicMemoryWorker::parse_memory_card(SAMPLE_LLM_RESPONSE);
        assert!(card.is_some(), "Should parse valid response");
        let card = card.unwrap();

        assert!(
            !card.date_context.is_empty(),
            "date_context should not be empty"
        );
        assert!(card.date_context.contains("infraestructura"));
        assert!(card.topics.contains("Podman"));
        assert!(card.topics.contains("PostgreSQL"));
        assert!(card.facts.contains("5432"));
        assert!(card.facts.contains("docker-compose.yml"));
        assert!(!card.synthesis.is_empty(), "synthesis should not be empty");
    }

    #[tokio::test]
    #[serial]
    async fn test_parse_memory_card_empty() {
        let card = EpisodicMemoryWorker::parse_memory_card("");
        assert!(card.is_none(), "Empty input should return None");
    }

    #[tokio::test]
    #[serial]
    async fn test_parse_memory_card_invalid() {
        let card =
            EpisodicMemoryWorker::parse_memory_card("This is just random text without sections.");
        assert!(card.is_none(), "Invalid input should return None");
    }
}

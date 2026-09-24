use rusqlite::Connection;
use uuid::Uuid;

use crate::db::vector;
use crate::embeddings::EmbeddingProvider;
use crate::models::{Memory, Message};

pub struct MemoryConsolidator {
    provider: Box<dyn EmbeddingProvider>,
}

impl MemoryConsolidator {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self {
        Self { provider }
    }

    /// Extract potential facts from a message and store as memories
    /// Simplified for F4: stores assistant messages as "conversation" memories
    pub async fn consolidate(
        &self,
        conn: &Connection,
        message: &Message,
    ) -> Result<Vec<Memory>, String> {
        if message.role != "assistant" || message.content.trim().is_empty() {
            return Ok(Vec::new());
        }

        // For F4, store assistant responses as memories for future reference
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO memories (id, profile_id, content, category, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, "default", message.content, "conversation", "assistant", now],
        )
        .map_err(|e | format!("Failed to store memory: {}", e))?;

        let memory = Memory {
            id: id.clone(),
            profile_id: "default".to_string(),
            content: message.content.clone(),
            category: "conversation".to_string(),
            source: "assistant".to_string(),
            embedding_id: None,
            created_at: now,
        };

        // Generate embedding for the new memory
        if let Ok(embedding) = self.provider.embed(&memory.content).await {
            let _ = vector::store_memory_embedding(conn, &memory.id, &embedding);
        }

        Ok(vec![memory])
    }
}

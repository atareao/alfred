use sqlx::SqlitePool;

use crate::db::vector;
use crate::embeddings::EmbeddingProvider;
use crate::models::{Memory, Message};

pub struct EmbeddingWorker {
    provider: Box<dyn EmbeddingProvider>,
}

impl EmbeddingWorker {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self {
        Self { provider }
    }

    /// Generate and store embedding for a message
    pub async fn index_message(&self, pool: &SqlitePool, message: &Message) -> Result<(), String> {
        if message.content.trim().is_empty() {
            return Ok(());
        }
        let embedding = self
            .provider
            .embed(&message.content)
            .await
            .map_err(|e| e.to_string())?;
        if let Err(e) = vector::store_message_embedding(pool, &message.id, &embedding)
            .await
            .map_err(|e| format!("Failed to store embedding: {}", e))
        {
            tracing::warn!(
                "Failed to generate embedding for message {}: {}",
                message.id,
                e
            );
        } else {
            tracing::debug!("Stored embedding for message {}", message.id);
        }
        Ok(())
    }

    /// Generate and store embedding for a memory
    pub async fn index_memory(&self, pool: &SqlitePool, memory: &Memory) -> Result<(), String> {
        if memory.content.trim().is_empty() {
            return Ok(());
        }
        let embedding = self
            .provider
            .embed(&memory.content)
            .await
            .map_err(|e| e.to_string())?;
        if let Err(e) = vector::store_memory_embedding(pool, &memory.id, &embedding)
            .await
            .map_err(|e| format!("Failed to store embedding: {}", e))
        {
            tracing::warn!(
                "Failed to generate embedding for memory {}: {}",
                memory.id,
                e
            );
        } else {
            tracing::debug!("Stored embedding for memory {}", memory.id);
        }
        Ok(())
    }
}

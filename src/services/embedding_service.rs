use rusqlite::Connection;

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
    pub async fn index_message(&self, conn: &Connection, message: &Message) -> Result<(), String> {
        if message.content.trim().is_empty() {
            return Ok(());
        }
        match self.provider.embed(&message.content).await {
            Ok(embedding) => {
                vector::store_message_embedding(conn, &message.id, &embedding)
                    .map_err(|e| format!("Failed to store embedding: {}", e))?;
                tracing::debug!("Stored embedding for message {}", message.id);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to generate embedding for message {}: {}",
                    message.id,
                    e
                );
            }
        }
        Ok(())
    }

    /// Generate and store embedding for a memory
    pub async fn index_memory(&self, conn: &Connection, memory: &Memory) -> Result<(), String> {
        if memory.content.trim().is_empty() {
            return Ok(());
        }
        match self.provider.embed(&memory.content).await {
            Ok(embedding) => {
                vector::store_memory_embedding(conn, &memory.id, &embedding)
                    .map_err(|e| format!("Failed to store embedding: {}", e))?;
                tracing::debug!("Stored embedding for memory {}", memory.id);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to generate embedding for memory {}: {}",
                    memory.id,
                    e
                );
            }
        }
        Ok(())
    }
}

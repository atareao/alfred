use serde::{Deserialize, Serialize};

/// Episodic memory card ("ficha") — a discrete remembered fact or observation.
///
/// Maps to the `memory` table created by migration 20260926000003.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub tokens_count: usize,
    pub created_at: String,
    pub metadata: serde_json::Value,
}

/// Input payload for creating a new episodic memory.
#[derive(Debug, Deserialize)]
pub struct CreateMemory {
    pub content: String,
    pub tokens_count: Option<usize>,
    pub metadata: Option<serde_json::Value>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub profile_id: String,
    pub content: String,
    pub category: String,
    pub source: String,
    pub embedding_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemory {
    pub profile_id: String,
    pub content: String,
    pub category: Option<String>,
    pub source: Option<String>,
}

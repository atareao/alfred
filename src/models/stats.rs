use serde::{Deserialize, Serialize};

/// Global aggregate statistics over all LLM requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    pub total_calls: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_cached_tokens: u64,
    pub total_reasoning_tokens: u64,
    pub total_cost: f64,
    pub total_errors: u64,
    pub avg_duration_ms: Option<f64>,
}

/// Per-model breakdown of LLM usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    pub model: String,
    pub calls: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub avg_duration_ms: Option<f64>,
    pub total_cached_tokens: u64,
    pub total_reasoning_tokens: u64,
}

/// Per-day time-series of LLM usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayStats {
    pub date: String,
    pub calls: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub total_cached_tokens: u64,
    pub total_reasoning_tokens: u64,
}

/// Count of calls per tool name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    pub tool: String,
    pub count: u64,
}

/// Row count for a single database table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSize {
    pub table: String,
    pub rows: u64,
}
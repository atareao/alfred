use sqlx::Row;
use sqlx::SqlitePool;

use crate::models::stats::{DayStats, ModelStats, StatsSummary, TableSize, ToolStats};

/// Repository for LLM usage statistics and administrative operations.
pub struct StatsRepo;

impl StatsRepo {
    /// Global aggregate summary over all LLM requests.
    pub async fn summary(pool: &SqlitePool) -> Result<StatsSummary, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*)                                               AS total_calls,
                COALESCE(SUM(prompt_tokens), 0)                        AS total_prompt_tokens,
                COALESCE(SUM(completion_tokens), 0)                    AS total_completion_tokens,
                COALESCE(SUM(total_tokens), 0)                         AS total_tokens,
                COALESCE(SUM(cached_tokens), 0)                        AS total_cached_tokens,
                COALESCE(SUM(reasoning_tokens), 0)                     AS total_reasoning_tokens,
                COALESCE(SUM(cost), 0.0)                               AS total_cost,
                COALESCE(SUM(CASE WHEN status != 'success' THEN 1 ELSE 0 END), 0) AS total_errors,
                AVG(duration_ms)                                       AS avg_duration_ms
            FROM llm_requests
            "#,
        )
        .fetch_one(pool)
        .await?;

        Ok(StatsSummary {
            total_calls: row.get::<i64, _>(0) as u64,
            total_prompt_tokens: row.get::<i64, _>(1) as u64,
            total_completion_tokens: row.get::<i64, _>(2) as u64,
            total_tokens: row.get::<i64, _>(3) as u64,
            total_cached_tokens: row.get::<i64, _>(4) as u64,
            total_reasoning_tokens: row.get::<i64, _>(5) as u64,
            total_cost: row.get::<f64, _>(6),
            total_errors: row.get::<i64, _>(7) as u64,
            avg_duration_ms: row.get::<Option<f64>, _>(8),
        })
    }

    /// Per-model breakdown, ordered by total cost descending.
    pub async fn by_model(pool: &SqlitePool) -> Result<Vec<ModelStats>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                model,
                COUNT(*)                        AS calls,
                SUM(total_tokens)               AS total_tokens,
                SUM(cost)                       AS total_cost,
                AVG(duration_ms)                AS avg_duration_ms,
                SUM(cached_tokens)              AS total_cached_tokens,
                SUM(reasoning_tokens)           AS total_reasoning_tokens
            FROM llm_requests
            GROUP BY model
            ORDER BY total_cost DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let stats = rows
            .iter()
            .map(|r| ModelStats {
                model: r.get(0),
                calls: r.get::<i64, _>(1) as u64,
                total_tokens: r.get::<i64, _>(2) as u64,
                total_cost: r.get::<f64, _>(3),
                avg_duration_ms: r.get::<Option<f64>, _>(4),
                total_cached_tokens: r.get::<i64, _>(5) as u64,
                total_reasoning_tokens: r.get::<i64, _>(6) as u64,
            })
            .collect();

        Ok(stats)
    }

    /// Daily time series for the last `days` days, ordered by date ascending.
    pub async fn by_day(pool: &SqlitePool, days: u32) -> Result<Vec<DayStats>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                DATE(created_at)                AS date,
                COUNT(*)                        AS calls,
                SUM(total_tokens)               AS total_tokens,
                SUM(cost)                       AS total_cost,
                SUM(cached_tokens)              AS total_cached_tokens,
                SUM(reasoning_tokens)           AS total_reasoning_tokens
            FROM llm_requests
            WHERE created_at >= datetime('now', '-' || ?1 || ' days')
            GROUP BY DATE(created_at)
            ORDER BY date ASC
            "#,
        )
        .bind(days as i64)
        .fetch_all(pool)
        .await?;

        let stats = rows
            .iter()
            .map(|r| DayStats {
                date: r.get(0),
                calls: r.get::<i64, _>(1) as u64,
                total_tokens: r.get::<i64, _>(2) as u64,
                total_cost: r.get::<f64, _>(3),
                total_cached_tokens: r.get::<i64, _>(4) as u64,
                total_reasoning_tokens: r.get::<i64, _>(5) as u64,
            })
            .collect();

        Ok(stats)
    }

    /// Frequency of tool calls across all LLM requests.
    ///
    /// Parses the `tool_calls` JSON column (an array of objects with a `name`
    /// field, e.g. `[{"name":"get_weather"}, {"name":"search_web"}]`).
    pub async fn tools_summary(pool: &SqlitePool) -> Result<Vec<ToolStats>, sqlx::Error> {
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT tool_calls FROM llm_requests WHERE tool_calls IS NOT NULL")
                .fetch_all(pool)
                .await?;

        let mut counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        for json_str in &rows {
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                for val in &arr {
                    if let Some(name) = val.get("name").and_then(|n| n.as_str()) {
                        *counts.entry(name.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut result: Vec<ToolStats> = counts
            .into_iter()
            .map(|(tool, count)| ToolStats { tool, count })
            .collect();

        // Sort by count descending for consistent output
        result.sort_by_key(|b| std::cmp::Reverse(b.count));

        Ok(result)
    }

    /// Row counts for all domain tables.
    ///
    /// Includes the primary content tables (excluding FTS virtual tables and
    /// internal helper tables).
    pub async fn db_sizes(pool: &SqlitePool) -> Result<Vec<TableSize>, sqlx::Error> {
        let tables = [
            "contacts",
            "events",
            "habit_logs",
            "habits",
            "llm_requests",
            "meal_plans",
            "memories",
            "message_embeddings",
            "memory_embeddings",
            "messages",
            "notes",
            "profiles",
            "reminders",
            "settings",
            "shopping_list",
            "tasks",
            "tools",
        ];

        let mut sizes = Vec::with_capacity(tables.len());

        for table in &tables {
            let sql = format!("SELECT COUNT(*) FROM \"{table}\"");
            let count: i64 = sqlx::query_scalar(&sql).fetch_one(pool).await?;
            sizes.push(TableSize {
                table: table.to_string(),
                rows: count as u64,
            });
        }

        Ok(sizes)
    }

    /// Export all LLM request records as a CSV string.
    ///
    /// The CSV includes a header row followed by one row per record.
    pub async fn export_csv(pool: &SqlitePool) -> Result<String, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, model, provider, profile_id,
                prompt_tokens, completion_tokens, total_tokens,
                cached_tokens, reasoning_tokens,
                cost, is_byok, duration_ms, cache_hit,
                status, error_message, tool_calls, created_at
            FROM llm_requests
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut csv = String::from(
            "id,model,provider,profile_id,prompt_tokens,completion_tokens,total_tokens,"
        );
        csv.push_str(
            "cached_tokens,reasoning_tokens,cost,is_byok,duration_ms,cache_hit,"
        );
        csv.push_str("status,error_message,tool_calls,created_at\n");

        for r in &rows {
            // Helper to quote a value for CSV
            let quote = |s: &str| {
                if s.contains(',') || s.contains('"') || s.contains('\n') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            };

            let id: String = r.get(0);
            let model: String = r.get(1);
            let provider: Option<String> = r.get(2);
            let profile_id: Option<String> = r.get(3);
            let prompt_tokens: i64 = r.get(4);
            let completion_tokens: i64 = r.get(5);
            let total_tokens: i64 = r.get(6);
            let cached_tokens: i64 = r.get(7);
            let reasoning_tokens: i64 = r.get(8);
            let cost: f64 = r.get(9);
            let is_byok: bool = r.get::<i64, _>(10) != 0;
            let duration_ms: Option<i64> = r.get(11);
            let cache_hit: bool = r.get::<i64, _>(12) != 0;
            let status: String = r.get(13);
            let error_message: Option<String> = r.get(14);
            let tool_calls: Option<String> = r.get(15);
            let created_at: String = r.get(16);

            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                quote(&id),
                quote(&model),
                provider.as_deref().unwrap_or(""),
                profile_id.as_deref().unwrap_or(""),
                prompt_tokens,
                completion_tokens,
                total_tokens,
                cached_tokens,
                reasoning_tokens,
                cost,
                if is_byok { 1 } else { 0 },
                duration_ms.map_or_else(String::new, |v| v.to_string()),
                if cache_hit { 1 } else { 0 },
                quote(&status),
                quote(error_message.as_deref().unwrap_or("")),
                quote(tool_calls.as_deref().unwrap_or("")),
                quote(&created_at),
            ));
        }

        Ok(csv)
    }

    /// Purge LLM request records older than `days` days.
    ///
    /// Returns the number of deleted rows.
    pub async fn purge_old(pool: &SqlitePool, days: u32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM llm_requests WHERE created_at < datetime('now', '-' || ?1 || ' days')",
        )
        .bind(days as i64)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Read the retention days setting.
    ///
    /// Returns the value of the `stats_retention_days` setting if present,
    /// otherwise returns the default of 30.
    pub async fn get_retention_days(pool: &SqlitePool) -> Result<u32, sqlx::Error> {
        let row: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'stats_retention_days'")
                .fetch_optional(pool)
                .await?;

        match row {
            Some(val) => val.parse::<u32>().or(Ok(30)),
            None => Ok(30),
        }
    }

    /// Save the retention days setting.
    pub async fn set_retention_days(pool: &SqlitePool, days: u32) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES ('stats_retention_days', ?1, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
        )
        .bind(days.to_string())
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    /// Helper to create an in-memory database with all migrations applied and a
    /// default profile inserted (needed for FK constraints on llm_requests).
    async fn setup() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("failed to create in-memory pool");

        crate::db::schema::run_migrations(&pool)
            .await
            .expect("failed to run migrations");

        // Seed a default profile for FK references
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&pool)
        .await
        .expect("failed to seed profile");

        pool
    }

    /// Convenience: insert a single LLM request row with the given parameters.
    #[allow(clippy::too_many_arguments)]
    async fn insert_request(
        pool: &SqlitePool,
        id: &str,
        model: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
        total_tokens: i64,
        cached_tokens: i64,
        reasoning_tokens: i64,
        cost: f64,
        duration_ms: Option<i64>,
        status: &str,
        error_message: Option<&str>,
        tool_calls: Option<&str>,
        created_at: Option<&str>,
    ) {
        let q = sqlx::query(
            "INSERT INTO llm_requests (id, model, provider, profile_id, prompt_tokens, completion_tokens, total_tokens, cached_tokens, reasoning_tokens, cost, is_byok, duration_ms, cache_hit, status, error_message, tool_calls, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, COALESCE(?17, datetime('now')))",
        )
        .bind(id)
        .bind(model)
        .bind(Option::<String>::None) // provider
        .bind("profile-1")
        .bind(prompt_tokens)
        .bind(completion_tokens)
        .bind(total_tokens)
        .bind(cached_tokens)
        .bind(reasoning_tokens)
        .bind(cost)
        .bind(0i64) // is_byok
        .bind(duration_ms)
        .bind(0i64) // cache_hit
        .bind(status)
        .bind(error_message)
        .bind(tool_calls)
        .bind(created_at);

        q.execute(pool).await.unwrap();
    }

    // ── 1. summary con datos variados ─────────────────────────────────────

    #[tokio::test]
    async fn test_summary_with_mixed_data() {
        let pool = setup().await;

        // Two successful requests
        insert_request(
            &pool, "req-1", "gpt-4o", 100, 50, 150, 10, 10, 0.01, Some(200),
            "success", None, None, None,
        )
        .await;
        insert_request(
            &pool, "req-2", "gpt-4o", 200, 100, 300, 20, 10, 0.02, Some(300),
            "success", None, None, None,
        )
        .await;
        // One error
        insert_request(
            &pool, "req-3", "claude-3", 0, 0, 0, 0, 0, 0.0, None,
            "error", Some("timeout"), None, None,
        )
        .await;

        let s = StatsRepo::summary(&pool).await.unwrap();

        assert_eq!(s.total_calls, 3);
        assert_eq!(s.total_prompt_tokens, 300);
        assert_eq!(s.total_completion_tokens, 150);
        assert_eq!(s.total_tokens, 450);
        assert_eq!(s.total_cached_tokens, 30);
        assert_eq!(s.total_reasoning_tokens, 20);
        assert!((s.total_cost - 0.03).abs() < f64::EPSILON);
        assert_eq!(s.total_errors, 1);
        // avg_duration_ms = (200 + 300 + NULL) / 2 = 250.0
        assert!((s.avg_duration_ms.unwrap() - 250.0).abs() < f64::EPSILON);
    }

    // ── 2. summary vacío ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_summary_empty() {
        let pool = setup().await;

        let s = StatsRepo::summary(&pool).await.unwrap();

        assert_eq!(s.total_calls, 0);
        assert_eq!(s.total_tokens, 0);
        assert_eq!(s.total_cost, 0.0);
        assert_eq!(s.total_errors, 0);
        assert!(s.avg_duration_ms.is_none());
    }

    // ── 3. by_model ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_by_model() {
        let pool = setup().await;

        // 2 calls gpt-4o (cost 0.02 + 0.01 = 0.03), 1 call claude-3 (cost 0.04)
        insert_request(
            &pool, "r1", "gpt-4o", 100, 50, 150, 10, 5, 0.02, Some(200),
            "success", None, None, None,
        )
        .await;
        insert_request(
            &pool, "r2", "gpt-4o", 50, 25, 75, 5, 3, 0.01, Some(100),
            "success", None, None, None,
        )
        .await;
        insert_request(
            &pool, "r3", "claude-3", 200, 100, 300, 20, 10, 0.04, Some(400),
            "success", None, None, None,
        )
        .await;

        let models = StatsRepo::by_model(&pool).await.unwrap();

        assert_eq!(models.len(), 2);
        // Ordered by total_cost DESC → claude-3 first (0.04) then gpt-4o (0.03)
        assert_eq!(models[0].model, "claude-3");
        assert_eq!(models[0].calls, 1);
        assert_eq!(models[0].total_cost, 0.04);
        assert_eq!(models[1].model, "gpt-4o");
        assert_eq!(models[1].calls, 2);
        assert!((models[1].total_cost - 0.03).abs() < f64::EPSILON);
    }

    // ── 4. by_day ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_by_day() {
        let pool = setup().await;

        // Insert one request per day for 7 days, ending yesterday
        for i in 1..=7 {
            let date = format!("2026-09-{:02}T10:00:00", 18 + i); // Sep 19..Sep 25
            let id = format!("day-req-{i}");
            insert_request(
                &pool, &id, "gpt-4o", 100, 50, 150, 10, 5, 0.01, Some(100),
                "success", None, None, Some(&date),
            )
            .await;
        }

        // by_day with days=30 should include all 7
        let days = StatsRepo::by_day(&pool, 30).await.unwrap();
        assert_eq!(days.len(), 7);
        // Ordered ascending
        for i in 0..days.len() - 1 {
            assert!(days[i].date <= days[i + 1].date);
        }
    }

    // ── 5. tools_summary ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_tools_summary() {
        let pool = setup().await;

        // 3 calls with get_weather
        for i in 0..3 {
            insert_request(
                &pool, &format!("tw-{i}"), "gpt-4o", 100, 50, 150, 0, 0, 0.01, None,
                "success", None, Some(r#"[{"name":"get_weather"}]"#), None,
            )
            .await;
        }
        // 2 calls with search_web
        for i in 0..2 {
            insert_request(
                &pool, &format!("ts-{i}"), "gpt-4o", 100, 50, 150, 0, 0, 0.01, None,
                "success", None, Some(r#"[{"name":"search_web"}]"#), None,
            )
            .await;
        }
        // 1 null tool_calls (should be ignored)
        insert_request(
            &pool, "t-null", "gpt-4o", 100, 50, 150, 0, 0, 0.01, None,
            "success", None, None, None,
        )
        .await;

        let tools = StatsRepo::tools_summary(&pool).await.unwrap();

        assert_eq!(tools.len(), 2);
        // Sorted by count desc: get_weather=3, search_web=2
        assert_eq!(tools[0].tool, "get_weather");
        assert_eq!(tools[0].count, 3);
        assert_eq!(tools[1].tool, "search_web");
        assert_eq!(tools[1].count, 2);
    }

    // ── 6. db_sizes ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_db_sizes() {
        let pool = setup().await;

        // Insert into various tables
        sqlx::query("INSERT INTO messages (id, role, content) VALUES ('m1', 'user', 'Hello')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO messages (id, role, content) VALUES ('m2', 'assistant', 'Hi')")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO memories (id, profile_id, content) VALUES ('mem1', 'profile-1', 'Memory')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let sizes = StatsRepo::db_sizes(&pool).await.unwrap();

        let messages_size = sizes.iter().find(|t| t.table == "messages").unwrap();
        assert_eq!(messages_size.rows, 2);

        let memories_size = sizes.iter().find(|t| t.table == "memories").unwrap();
        assert_eq!(memories_size.rows, 1);

        let profiles_size = sizes.iter().find(|t| t.table == "profiles").unwrap();
        assert_eq!(profiles_size.rows, 1);
    }

    // ── 7. export_csv ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_export_csv() {
        let pool = setup().await;

        insert_request(
            &pool, "csv-1", "gpt-4o", 100, 50, 150, 10, 5, 0.01, Some(200),
            "success", None, Some(r#"[{"name":"get_weather"}]"#),
            Some("2026-09-24T10:00:00"),
        )
        .await;
        insert_request(
            &pool, "csv-2", "claude-3", 200, 100, 300, 20, 10, 0.02, None,
            "error", Some("timeout"), None,
            Some("2026-09-25T12:00:00"),
        )
        .await;

        let csv = StatsRepo::export_csv(&pool).await.unwrap();

        // Should have a header and 2 data lines
        let lines: Vec<&str> = csv.trim().lines().collect();
        assert!(lines.len() >= 3, "expected header + 2 rows, got {}", lines.len());

        let header = lines[0];
        assert!(header.starts_with("id,"));
        assert!(header.contains("model"));
        assert!(header.contains("status"));

        // Check first data row
        let row1 = lines[1];
        assert!(row1.starts_with("csv-1,"));
        assert!(row1.contains("gpt-4o"));

        // Check second row
        let row2 = lines[2];
        assert!(row2.starts_with("csv-2,"));
        assert!(row2.contains("claude-3"));
        assert!(row2.contains("timeout"));
    }

    // ── 8. purge_old ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_purge_old() {
        let pool = setup().await;

        // 10 old records (60 days ago)
        for i in 0..10 {
            let id = format!("old-{i}");
            insert_request(
                &pool, &id, "gpt-4o", 10, 5, 15, 0, 0, 0.001, None,
                "success", None, None, Some("2026-07-01T00:00:00"),
            )
            .await;
        }

        // 10 new records (today)
        for i in 0..10 {
            let id = format!("new-{i}");
            insert_request(
                &pool, &id, "gpt-4o", 10, 5, 15, 0, 0, 0.001, None,
                "success", None, None, None,
            )
            .await;
        }

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM llm_requests")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 20);

        // Purge records older than 30 days → should remove the 10 old ones
        let deleted = StatsRepo::purge_old(&pool, 30).await.unwrap();
        assert_eq!(deleted, 10);

        let remaining: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM llm_requests")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(remaining, 10);
    }

    // ── 9. get_retention_days ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_retention_days_with_setting() {
        let pool = setup().await;

        // Insert setting
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES ('stats_retention_days', '45')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let days = StatsRepo::get_retention_days(&pool).await.unwrap();
        assert_eq!(days, 45);
    }

    #[tokio::test]
    async fn test_get_retention_days_default() {
        let pool = setup().await;

        // No setting inserted → should default to 30
        let days = StatsRepo::get_retention_days(&pool).await.unwrap();
        assert_eq!(days, 30);
    }

    // ── 10. set_retention_days ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_set_retention_days() {
        let pool = setup().await;

        StatsRepo::set_retention_days(&pool, 60).await.unwrap();

        let days = StatsRepo::get_retention_days(&pool).await.unwrap();
        assert_eq!(days, 60);

        // Update existing
        StatsRepo::set_retention_days(&pool, 90).await.unwrap();
        let days = StatsRepo::get_retention_days(&pool).await.unwrap();
        assert_eq!(days, 90);
    }
}
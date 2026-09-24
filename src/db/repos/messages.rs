use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::Value;
use uuid::Uuid;

use crate::models::message::estimate_tokens;
use crate::models::Message;

pub struct MessagesRepo;

impl MessagesRepo {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        conn: &Connection,
        conversation_id: &str,
        role: &str,
        content: &str,
        tool_calls: Option<&Value>,
        tool_results: Option<&Value>,
        collapse_threshold: usize,
        on_collapse_needed: Option<Box<dyn Fn(String)>>,
    ) -> Result<Message, rusqlite::Error> {
        // Verify conversation exists
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM conversations WHERE id = ?1",
            params![conversation_id],
            |row| row.get::<_, i64>(0),
        )? > 0;
        if !exists {
            return Err(rusqlite::Error::InvalidParameterName(
                "conversation not found".into(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tc = tool_calls.map(|v| v.to_string());
        let tr = tool_results.map(|v| v.to_string());
        let tokens = estimate_tokens(content);

        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, tool_calls, tool_results, tokens_count, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, conversation_id, role, content, tc, tr, tokens, now],
        )?;

        // Update conversation's updated_at
        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now, conversation_id],
        )?;

        // If tokens exceed threshold, invoke collapse callback
        if tokens >= collapse_threshold {
            if let Some(ref callback) = on_collapse_needed {
                callback(id.clone());
            }
        }

        Ok(Message {
            id,
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: tool_calls.cloned(),
            tool_results: tool_results.cloned(),
            tokens_count: tokens,
            collapsed_content: None,
            collapsed_tokens_count: 0,
            is_indexed: false,
            summary_ref: None,
            created_at: now,
        })
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Message>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, tool_calls, tool_results, \
             tokens_count, collapsed_content, collapsed_tokens_count, is_indexed, summary_ref, created_at \
             FROM messages WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => {
                let tc: Option<String> = row.get(4)?;
                let tr: Option<String> = row.get(5)?;
                Ok(Some(Message {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    tool_calls: tc.and_then(|s| serde_json::from_str(&s).ok()),
                    tool_results: tr.and_then(|s| serde_json::from_str(&s).ok()),
                    tokens_count: row.get(6)?,
                    collapsed_content: row.get(7)?,
                    collapsed_tokens_count: row.get(8)?,
                    is_indexed: row.get(9)?,
                    summary_ref: row.get(10)?,
                    created_at: row.get(11)?,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn list_by_conversation(
        conn: &Connection,
        conversation_id: &str,
        limit: i64,
        cursor: Option<&str>,
    ) -> Result<(Vec<Message>, Option<String>), rusqlite::Error> {
        let actual_limit = limit.clamp(1, 100);
        let (query, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match cursor {
            Some(c) => (
                "SELECT id, conversation_id, role, content, tool_calls, tool_results, \
                 tokens_count, collapsed_content, collapsed_tokens_count, is_indexed, summary_ref, created_at \
                 FROM messages WHERE conversation_id = ?1 AND created_at > ?2 \
                 ORDER BY created_at ASC LIMIT ?3"
                    .into(),
                vec![
                    Box::new(conversation_id.to_string()),
                    Box::new(c.to_string()),
                    Box::new(actual_limit + 1),
                ],
            ),
            None => (
                "SELECT id, conversation_id, role, content, tool_calls, tool_results, \
                 tokens_count, collapsed_content, collapsed_tokens_count, is_indexed, summary_ref, created_at \
                 FROM messages WHERE conversation_id = ?1 \
                 ORDER BY created_at ASC LIMIT ?2"
                    .into(),
                vec![
                    Box::new(conversation_id.to_string()),
                    Box::new(actual_limit + 1),
                ],
            ),
        };

        let mut stmt = conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;

        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            let tc: Option<String> = row.get(4)?;
            let tr: Option<String> = row.get(5)?;
            items.push(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: tc.and_then(|s| serde_json::from_str(&s).ok()),
                tool_results: tr.and_then(|s| serde_json::from_str(&s).ok()),
                tokens_count: row.get(6)?,
                collapsed_content: row.get(7)?,
                collapsed_tokens_count: row.get(8)?,
                is_indexed: row.get(9)?,
                summary_ref: row.get(10)?,
                created_at: row.get(11)?,
            });
        }

        let has_more = items.len() > actual_limit as usize;
        let data: Vec<Message> = if has_more {
            items[..actual_limit as usize].to_vec()
        } else {
            items
        };
        let next_cursor = if has_more {
            data.last().map(|m| m.created_at.clone())
        } else {
            None
        };
        Ok((data, next_cursor))
    }

    /// Return the last N messages in chronological order (not the first N).
    /// Used for rehydrating the session window at server startup.
    pub fn list_recent(
        conn: &Connection,
        conversation_id: &str,
        limit: i64,
    ) -> Result<Vec<Message>, rusqlite::Error> {
        let actual_limit = limit.clamp(1, 200);
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, tool_calls, tool_results, \
             tokens_count, collapsed_content, collapsed_tokens_count, is_indexed, summary_ref, created_at \
             FROM messages WHERE conversation_id = ?1 \
             ORDER BY created_at DESC LIMIT ?2",
        )?;
        let mut rows = stmt.query(params![conversation_id, actual_limit])?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            let tc: Option<String> = row.get(4)?;
            let tr: Option<String> = row.get(5)?;
            items.push(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: tc.and_then(|s| serde_json::from_str(&s).ok()),
                tool_results: tr.and_then(|s| serde_json::from_str(&s).ok()),
                tokens_count: row.get(6)?,
                collapsed_content: row.get(7)?,
                collapsed_tokens_count: row.get(8)?,
                is_indexed: row.get(9)?,
                summary_ref: row.get(10)?,
                created_at: row.get(11)?,
            });
        }
        items.reverse(); // back to chronological order
        Ok(items)
    }

    pub fn list_by_token_budget(
        conn: &Connection,
        conversation_id: &str,
        max_tokens: usize,
    ) -> Result<Vec<Message>, rusqlite::Error> {
        // SQL window function query that:
        // 1. Uses COALESCE(collapsed_tokens_count, tokens_count) as effective_tokens
        // 2. Uses COALESCE(collapsed_content, content) as effective_content
        // 3. SUM OVER (ORDER BY created_at DESC) for cumulative tokens
        // 4. Filters WHERE cumulative_tokens <= max_tokens
        // 5. Orders by created_at ASC
        let mut stmt = conn.prepare(
            "WITH RankedMessages AS (
                SELECT id, conversation_id, role,
                       CASE WHEN collapsed_content IS NOT NULL THEN collapsed_content ELSE content END AS effective_content,
                       CASE WHEN collapsed_content IS NOT NULL THEN collapsed_tokens_count ELSE tokens_count END AS effective_tokens,
                       tool_calls, tool_results,
                       collapsed_content, collapsed_tokens_count,
                       is_indexed, summary_ref, created_at,
                       SUM(CASE WHEN collapsed_content IS NOT NULL THEN collapsed_tokens_count ELSE tokens_count END)
                           OVER (ORDER BY created_at DESC ROWS UNBOUNDED PRECEDING) AS cumulative_tokens
                FROM messages
                WHERE conversation_id = ?1
            )
            SELECT id, conversation_id, role, effective_content,
                   tool_calls, tool_results,
                   effective_tokens, collapsed_content, collapsed_tokens_count,
                   is_indexed, summary_ref, created_at
            FROM RankedMessages
            WHERE cumulative_tokens <= ?2
            ORDER BY created_at ASC",
        )?;

        let max_tokens_i64 = max_tokens as i64;
        let rows = stmt.query_map(params![conversation_id, max_tokens_i64], |row| {
            let tc: Option<String> = row.get(4)?;
            let tr: Option<String> = row.get(5)?;
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: tc.and_then(|s| serde_json::from_str(&s).ok()),
                tool_results: tr.and_then(|s| serde_json::from_str(&s).ok()),
                tokens_count: row.get(6)?,
                collapsed_content: row.get(7)?,
                collapsed_tokens_count: row.get(8)?,
                is_indexed: row.get(9)?,
                summary_ref: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    pub fn delete_by_conversation(
        conn: &Connection,
        conversation_id: &str,
    ) -> Result<usize, rusqlite::Error> {
        conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![conversation_id],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repos::conversations::ConversationsRepo;
    use crate::db::schema::run_migrations;
    use serde_json::json;

    fn setup_with_conversation() -> (Connection, String) {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let conv = ConversationsRepo::create(&conn, "Test Conv").unwrap();
        (conn, conv.id)
    }

    #[test]
    fn test_create_message() {
        let (conn, conv_id) = setup_with_conversation();
        let msg =
            MessagesRepo::create(&conn, &conv_id, "user", "Hello", None, None, 2000, None).unwrap();
        assert!(!msg.id.is_empty());
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.conversation_id, conv_id);
    }

    #[test]
    fn test_create_message_with_tool_calls() {
        let (conn, conv_id) = setup_with_conversation();
        let tool_calls = json!([{"name": "get_weather", "args": {"city": "Madrid"}}]);
        let msg = MessagesRepo::create(
            &conn,
            &conv_id,
            "assistant",
            "Let me check",
            Some(&tool_calls),
            None,
            2000,
            None,
        )
        .unwrap();
        assert!(msg.tool_calls.is_some());
    }

    #[test]
    fn test_create_message_invalid_conversation() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let result = MessagesRepo::create(
            &conn,
            "nonexistent",
            "user",
            "Hello",
            None,
            None,
            2000,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_find_by_id_not_found() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let found = MessagesRepo::find_by_id(&conn, "nonexistent").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_list_by_conversation() {
        let (conn, conv_id) = setup_with_conversation();
        MessagesRepo::create(&conn, &conv_id, "user", "First", None, None, 2000, None).unwrap();
        MessagesRepo::create(
            &conn,
            &conv_id,
            "assistant",
            "Second",
            None,
            None,
            2000,
            None,
        )
        .unwrap();
        let (msgs, cursor) = MessagesRepo::list_by_conversation(&conn, &conv_id, 10, None).unwrap();
        assert_eq!(msgs.len(), 2);
        assert!(cursor.is_none());
    }

    #[test]
    fn test_list_by_conversation_with_pagination() {
        let (conn, conv_id) = setup_with_conversation();
        MessagesRepo::create(&conn, &conv_id, "user", "Msg 1", None, None, 2000, None).unwrap();
        MessagesRepo::create(&conn, &conv_id, "user", "Msg 2", None, None, 2000, None).unwrap();
        MessagesRepo::create(&conn, &conv_id, "user", "Msg 3", None, None, 2000, None).unwrap();

        let (page1, cursor) = MessagesRepo::list_by_conversation(&conn, &conv_id, 2, None).unwrap();
        assert_eq!(page1.len(), 2);
        assert!(cursor.is_some());

        let (page2, cursor2) =
            MessagesRepo::list_by_conversation(&conn, &conv_id, 2, cursor.as_deref()).unwrap();
        assert_eq!(page2.len(), 1);
        assert!(cursor2.is_none());
    }

    #[test]
    fn test_delete_by_conversation() {
        let (conn, conv_id) = setup_with_conversation();
        MessagesRepo::create(&conn, &conv_id, "user", "Msg", None, None, 2000, None).unwrap();
        let deleted = MessagesRepo::delete_by_conversation(&conn, &conv_id).unwrap();
        assert_eq!(deleted, 1);
    }

    #[test]
    fn test_list_recent_returns_chronological_order() {
        let (conn, conv_id) = setup_with_conversation();
        MessagesRepo::create(&conn, &conv_id, "user", "First", None, None, 2000, None).unwrap();
        MessagesRepo::create(
            &conn,
            &conv_id,
            "assistant",
            "Second",
            None,
            None,
            2000,
            None,
        )
        .unwrap();
        MessagesRepo::create(&conn, &conv_id, "user", "Third", None, None, 2000, None).unwrap();
        let recent = MessagesRepo::list_recent(&conn, &conv_id, 10).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "First");
        assert_eq!(recent[1].content, "Second");
        assert_eq!(recent[2].content, "Third");
    }

    #[test]
    fn test_list_recent_respects_limit() {
        let (conn, conv_id) = setup_with_conversation();
        MessagesRepo::create(&conn, &conv_id, "user", "A", None, None, 2000, None).unwrap();
        MessagesRepo::create(&conn, &conv_id, "user", "B", None, None, 2000, None).unwrap();
        MessagesRepo::create(&conn, &conv_id, "user", "C", None, None, 2000, None).unwrap();
        let recent = MessagesRepo::list_recent(&conn, &conv_id, 2).unwrap();
        assert_eq!(recent.len(), 2);
        // The 2 most recent are "B" and "C", reversed to chronological => "B", "C"
        assert_eq!(recent[0].content, "B");
        assert_eq!(recent[1].content, "C");
    }

    #[test]
    fn test_list_recent_empty_conversation() {
        let (conn, conv_id) = setup_with_conversation();
        let recent = MessagesRepo::list_recent(&conn, &conv_id, 10).unwrap();
        assert!(recent.is_empty());
    }

    /// After creating a message, `tokens_count` must be automatically
    /// computed from the content. (RED: currently returns 0)
    #[test]
    fn test_create_message_computes_tokens() {
        let (conn, conv_id) = setup_with_conversation();
        let content = "Hello, how are you? This is a test message.";
        let msg =
            MessagesRepo::create(&conn, &conv_id, "user", content, None, None, 2000, None).unwrap();
        assert!(
            msg.tokens_count > 0,
            "tokens_count should be > 0, got {}",
            msg.tokens_count
        );
        let expected = crate::models::message::estimate_tokens(content);
        assert_eq!(
            msg.tokens_count, expected,
            "tokens_count should equal estimate_tokens(content)"
        );
    }

    /// A short message (below collapse threshold) must NOT have
    /// a collapsed_content. (RED: tokens_count is 0 so the test
    /// cannot verify the threshold logic yet)
    #[test]
    fn test_create_message_short_no_collapse() {
        let (conn, conv_id) = setup_with_conversation();
        let content = "Short message.";
        let msg =
            MessagesRepo::create(&conn, &conv_id, "user", content, None, None, 2000, None).unwrap();
        assert!(
            msg.collapsed_content.is_none(),
            "Short messages should not have collapsed_content"
        );
        let estimated = crate::models::message::estimate_tokens(content);
        assert!(
            estimated < 2000,
            "Short message should be below collapse threshold"
        );
    }

    #[test]
    fn test_create_message_long_triggers_collapse() {
        let (conn, conv_id) = setup_with_conversation();
        // 8000 chars → ~2289 tokens (well above 2000 threshold)
        let long_content = "x".repeat(8000);

        let collapse_requested = std::sync::Arc::new(std::sync::Mutex::new(false));
        let captured_id = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let cr_clone = collapse_requested.clone();
        let ci_clone = captured_id.clone();

        let callback: Option<Box<dyn Fn(String)>> = Some(Box::new(move |msg_id: String| {
            *cr_clone.lock().unwrap() = true;
            *ci_clone.lock().unwrap() = msg_id;
        }));

        let msg = MessagesRepo::create(
            &conn,
            &conv_id,
            "user",
            &long_content,
            None,
            None,
            2000,
            callback,
        )
        .unwrap();

        // The estimated tokens must exceed the threshold
        let estimated = crate::models::message::estimate_tokens(&long_content);
        assert!(
            estimated > 2000,
            "Long content should exceed collapse threshold (estimated={})",
            estimated
        );
        assert!(
            msg.tokens_count >= estimated,
            "tokens_count should be >= estimated tokens"
        );
        // When the threshold is exceeded, the collapse callback must fire
        assert!(
            *collapse_requested.lock().unwrap(),
            "Collapse callback should have been invoked for long message"
        );
        assert_eq!(
            *captured_id.lock().unwrap(),
            msg.id,
            "Callback should receive the message id"
        );
    }

    // ── list_by_token_budget tests ──────────────────────────────────────────

    #[test]
    fn test_list_by_token_budget_selects_within_budget() {
        let (conn, conv_id) = setup_with_conversation();

        // ~1000 tokens: ceil(3486/3.5) + 4 = 1000
        let content_1k = "a".repeat(3486);
        // ~2000 tokens: ceil(6986/3.5) + 4 = 2000
        let content_2k = "a".repeat(6986);

        let msg1 = MessagesRepo::create(
            &conn,
            &conv_id,
            "user",
            &content_1k,
            None,
            None,
            99999,
            None,
        )
        .unwrap();
        let msg2 = MessagesRepo::create(
            &conn,
            &conv_id,
            "user",
            &content_2k,
            None,
            None,
            99999,
            None,
        )
        .unwrap();
        let msg3 = MessagesRepo::create(
            &conn,
            &conv_id,
            "user",
            &content_1k,
            None,
            None,
            99999,
            None,
        )
        .unwrap();

        let result = MessagesRepo::list_by_token_budget(&conn, &conv_id, 3500).unwrap();

        // Should return the 2 most recent messages: msg2 (2000) + msg3 (1000) = 3000 ≤ 3500
        assert_eq!(result.len(), 2, "Should keep 2 messages within budget");
        assert_eq!(result[0].id, msg2.id, "Oldest kept message should be msg2");
        assert_eq!(result[1].id, msg3.id, "Newest kept message should be msg3");
        // msg1 (1000 tokens) excluded because adding it would exceed 3500
        let ids: Vec<&str> = result.iter().map(|m| m.id.as_str()).collect();
        assert!(
            !ids.contains(&msg1.id.as_str()),
            "Oldest message (msg1) should be excluded"
        );
    }

    #[test]
    fn test_list_by_token_budget_all_fit() {
        let (conn, conv_id) = setup_with_conversation();

        // ~500 tokens: ceil(1736/3.5) + 4 = 500
        let content_500 = "a".repeat(1736);

        let msg1 = MessagesRepo::create(
            &conn,
            &conv_id,
            "user",
            &content_500,
            None,
            None,
            99999,
            None,
        )
        .unwrap();
        let msg2 = MessagesRepo::create(
            &conn,
            &conv_id,
            "user",
            &content_500,
            None,
            None,
            99999,
            None,
        )
        .unwrap();

        let result = MessagesRepo::list_by_token_budget(&conn, &conv_id, 2000).unwrap();

        assert_eq!(result.len(), 2, "Both messages should fit within budget");
        assert_eq!(result[0].id, msg1.id);
        assert_eq!(result[1].id, msg2.id);
    }

    #[test]
    fn test_list_by_token_budget_uses_collapsed_tokens() {
        let (conn, conv_id) = setup_with_conversation();

        // Create a message with ~3000 tokens
        let content_3k = "a".repeat(10486); // ceil(10486/3.5) + 4 = 3000
        let msg = MessagesRepo::create(
            &conn,
            &conv_id,
            "user",
            &content_3k,
            None,
            None,
            99999,
            None,
        )
        .unwrap();

        // Simulate collapsing: set collapsed fields via raw SQL
        let collapsed_text = "Short collapsed summary";
        conn.execute(
            "UPDATE messages SET collapsed_content = ?1, collapsed_tokens_count = ?2 WHERE id = ?3",
            rusqlite::params![collapsed_text, 200, msg.id],
        )
        .unwrap();

        // max_tokens=500, collapsed_tokens_count=200 ≤ 500 → included
        let result = MessagesRepo::list_by_token_budget(&conn, &conv_id, 500).unwrap();

        assert_eq!(result.len(), 1, "Collapsed message should be included");
        assert_eq!(
            result[0].content, collapsed_text,
            "Returned content should be the collapsed content"
        );
        assert_eq!(
            result[0].collapsed_tokens_count, 200,
            "Collapsed tokens count should be preserved"
        );
    }

    #[test]
    fn test_list_by_token_budget_zero_budget() {
        let (conn, conv_id) = setup_with_conversation();

        let content = "Some message content";
        MessagesRepo::create(&conn, &conv_id, "user", content, None, None, 99999, None).unwrap();
        MessagesRepo::create(
            &conn,
            &conv_id,
            "assistant",
            "response",
            None,
            None,
            99999,
            None,
        )
        .unwrap();

        let result = MessagesRepo::list_by_token_budget(&conn, &conv_id, 0).unwrap();

        assert!(result.is_empty(), "Zero budget should return empty list");
    }

    #[test]
    fn test_list_by_token_budget_empty_conversation() {
        let (conn, conv_id) = setup_with_conversation();

        let result = MessagesRepo::list_by_token_budget(&conn, &conv_id, 10000).unwrap();

        assert!(
            result.is_empty(),
            "Empty conversation should return empty list"
        );
    }
}

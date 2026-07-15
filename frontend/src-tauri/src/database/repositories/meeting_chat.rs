use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub meeting_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub session_id: String,
}

pub struct MeetingChatRepository;

impl MeetingChatRepository {
    /// Save a single chat message
    pub async fn save_message(
        pool: &SqlitePool,
        meeting_id: &str,
        role: &str,
        content: &str,
        session_id: &str,
    ) -> Result<ChatMessage, sqlx::Error> {
        let ts = chrono::Utc::now().timestamp_millis();

        sqlx::query_as::<_, ChatMessage>(
            "INSERT INTO meeting_chat_messages (meeting_id, role, content, timestamp, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5)
             RETURNING id, meeting_id, role, content, timestamp, session_id",
        )
        .bind(meeting_id)
        .bind(role)
        .bind(content)
        .bind(ts)
        .bind(session_id)
        .fetch_one(pool)
        .await
    }

    /// Get all messages for a meeting's chat session, ordered by timestamp
    pub async fn get_session_messages(
        pool: &SqlitePool,
        meeting_id: &str,
        session_id: &str,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT id, meeting_id, role, content, timestamp, session_id
             FROM meeting_chat_messages
             WHERE meeting_id = ?1 AND session_id = ?2
             ORDER BY timestamp ASC",
        )
        .bind(meeting_id)
        .bind(session_id)
        .fetch_all(pool)
        .await
    }

    /// Get all chat session IDs for a meeting (distinct, newest first)
    pub async fn get_sessions(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<(String, i64)>, sqlx::Error> {
        // Returns (session_id, message_count)
        sqlx::query_as(
            "SELECT session_id, COUNT(*) as cnt
             FROM meeting_chat_messages
             WHERE meeting_id = ?1
             GROUP BY session_id
             ORDER BY MAX(timestamp) DESC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await
    }

    /// Get all messages for a meeting (across all sessions), ordered by timestamp
    pub async fn get_all_messages(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT id, meeting_id, role, content, timestamp, session_id
             FROM meeting_chat_messages
             WHERE meeting_id = ?1
             ORDER BY timestamp ASC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await
    }

    /// Delete a specific chat session
    pub async fn delete_session(
        pool: &SqlitePool,
        meeting_id: &str,
        session_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM meeting_chat_messages WHERE meeting_id = ?1 AND session_id = ?2",
        )
        .bind(meeting_id)
        .bind(session_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

use sqlx::SqlitePool;

use super::super::models::MeetingCustomNote;

pub struct MeetingCustomNotesRepository;

impl MeetingCustomNotesRepository {
    /// Save a new custom note for a meeting
    pub async fn save_note(
        pool: &SqlitePool,
        meeting_id: &str,
        timestamp: i64,
        note_text: &str,
    ) -> Result<MeetingCustomNote, sqlx::Error> {
        // Ensure meeting exists in meetings table first to satisfy foreign key constraint.
        // Use INSERT OR IGNORE to create a placeholder if it doesn't exist yet.
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT OR IGNORE INTO meetings (id, title, created_at, updated_at, folder_path)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(meeting_id)
        .bind("New Call")
        .bind(now)
        .bind(now)
        .bind(None::<String>)
        .execute(pool)
        .await?;

        let result = sqlx::query_as::<_, MeetingCustomNote>(
            "INSERT INTO meeting_custom_notes (meeting_id, timestamp, note_text)
             VALUES (?1, ?2, ?3)
             RETURNING id, meeting_id, timestamp, note_text",
        )
        .bind(meeting_id)
        .bind(timestamp)
        .bind(note_text)
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get all custom notes for a meeting, ordered by timestamp
    pub async fn get_notes_for_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<MeetingCustomNote>, sqlx::Error> {
        sqlx::query_as::<_, MeetingCustomNote>(
            "SELECT id, meeting_id, timestamp, note_text
             FROM meeting_custom_notes
             WHERE meeting_id = ?1
             ORDER BY timestamp ASC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await
    }

    /// Delete a specific note by ID
    pub async fn delete_note(
        pool: &SqlitePool,
        note_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM meeting_custom_notes WHERE id = ?1")
            .bind(note_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

use sqlx::SqlitePool;

pub struct MeetingMarkdownNotesRepository;

impl MeetingMarkdownNotesRepository {
    /// Save or update the markdown note for a meeting (upsert).
    /// Ensures the meeting row exists first to satisfy the foreign key constraint.
    pub async fn save_note(
        pool: &SqlitePool,
        meeting_id: &str,
        markdown: &str,
    ) -> Result<(), sqlx::Error> {
        // Ensure meeting exists (foreign key requirement)
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT OR IGNORE INTO meetings (id, title, created_at, updated_at, folder_path)
             VALUES (?1, ?2, ?3, ?3, ?4)",
        )
        .bind(meeting_id)
        .bind("New Call")
        .bind(&now)
        .bind(None::<String>)
        .execute(pool)
        .await?;

        let now_rfc = now.to_rfc3339();
        sqlx::query(
            "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(meeting_id) DO UPDATE SET
                notes_markdown = excluded.notes_markdown,
                updated_at = excluded.updated_at",
        )
        .bind(meeting_id)
        .bind(markdown)
        .bind(&now_rfc)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get the markdown note for a meeting
    pub async fn get_note(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT notes_markdown FROM meeting_notes WHERE meeting_id = ?1",
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| r.0))
    }
}

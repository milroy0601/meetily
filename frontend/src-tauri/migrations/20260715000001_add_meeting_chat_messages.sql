-- Add meeting_chat_messages table for persistent chat history per meeting
CREATE TABLE IF NOT EXISTS meeting_chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_meeting ON meeting_chat_messages(meeting_id, session_id, timestamp);

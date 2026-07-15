-- Add meeting_custom_notes table for storing individual timestamped user notes during meetings
-- Separate from the existing meeting_notes table which stores a single markdown/json note per meeting
CREATE TABLE IF NOT EXISTS meeting_custom_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    note_text TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

-- Create index for faster lookups by meeting_id
CREATE INDEX IF NOT EXISTS idx_meeting_custom_notes_meeting_id ON meeting_custom_notes(meeting_id);

-- Create index for timestamp-based ordering
CREATE INDEX IF NOT EXISTS idx_meeting_custom_notes_timestamp ON meeting_custom_notes(meeting_id, timestamp);

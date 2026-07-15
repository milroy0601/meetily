'use client';

import { useState, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { StickyNote, Loader2 } from 'lucide-react';

interface MeetingNotesPanelProps {
  meetingId: string;
  recordingStartTime?: number;
}

/**
 * MeetingNotesPanel - Single continuous markdown note for a meeting.
 * Auto-saves on debounce during recording. Editable after recording too.
 */
export function MeetingNotesPanel({ meetingId }: MeetingNotesPanelProps) {
  const [markdown, setMarkdown] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [lastSaved, setLastSaved] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const loadedRef = useRef(false);

  // Load existing note on mount
  useEffect(() => {
    if (loadedRef.current || !meetingId) return;
    loadedRef.current = true;

    const loadNote = async () => {
      try {
        const note = await invoke<string | null>('get_meeting_markdown_note', {
          meetingId,
        });
        if (note) {
          setMarkdown(note);
          setLastSaved(note);
        }
      } catch (error) {
        console.warn('Failed to load meeting note:', error);
      }
    };
    loadNote();
  }, [meetingId]);

  // Debounced auto-save: 1.5s after user stops typing
  const debouncedSave = useCallback(
    (text: string) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);

      debounceRef.current = setTimeout(async () => {
        if (text === lastSaved) return;
        try {
          setIsSaving(true);
          await invoke('save_meeting_markdown_note', {
            meetingId,
            markdown: text,
          });
          setLastSaved(text);
        } catch (error) {
          console.error('Failed to save note:', error);
        } finally {
          setIsSaving(false);
        }
      }, 1500);
    },
    [meetingId, lastSaved],
  );

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const text = e.target.value;
    setMarkdown(text);
    debouncedSave(text);
  };

  // Cancel pending save on unmount
  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  return (
    <div className="flex flex-col h-full">
      <div className="sticky top-0 z-10 bg-white px-4 py-3 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <StickyNote className="w-4 h-4 text-yellow-600" />
            <h3 className="text-sm font-semibold text-gray-700">Notes</h3>
          </div>
          {isSaving && (
            <span className="text-xs text-gray-400 flex items-center gap-1">
              <Loader2 className="w-3 h-3 animate-spin" /> Saving...
            </span>
          )}
          {!isSaving && lastSaved != null && (
            <span className="text-xs text-green-500">Saved</span>
          )}
        </div>
      </div>
      <textarea
        className="flex-1 w-full p-4 text-sm border-0 resize-none focus:outline-none focus:ring-0"
        placeholder="Write your meeting notes in markdown..."
        value={markdown}
        onChange={handleChange}
      />
    </div>
  );
}

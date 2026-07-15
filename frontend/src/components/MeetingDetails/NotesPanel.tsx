'use client';

import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { StickyNote, Loader2 } from 'lucide-react';

interface NotesPanelProps {
  meetingId: string;
}

export function NotesPanel({ meetingId }: NotesPanelProps) {
  const [markdown, setMarkdown] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const loadedRef = useRef(false);

  useEffect(() => {
    if (loadedRef.current) return;
    loadedRef.current = true;
    (async () => {
      try {
        const note = await invoke<string | null>('get_meeting_markdown_note', { meetingId });
        if (note) setMarkdown(note);
      } catch { /* ignore */ }
    })();
  }, [meetingId]);

  const handleChange = (text: string) => {
    setMarkdown(text);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      try {
        setIsSaving(true);
        await invoke('save_meeting_markdown_note', { meetingId, markdown: text });
      } catch (e) { console.error(e); }
      finally { setIsSaving(false); }
    }, 1000);
  };

  useEffect(() => () => { if (debounceRef.current) clearTimeout(debounceRef.current); }, []);

  return (
    <div className="flex flex-col h-full bg-white">
      <div className="sticky top-0 z-10 bg-white px-4 py-3 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <StickyNote className="w-4 h-4 text-yellow-600" />
            <h3 className="text-sm font-semibold text-gray-700">Meeting Notes</h3>
          </div>
          {isSaving && (
            <span className="text-xs text-gray-400 flex items-center gap-1">
              <Loader2 className="w-3 h-3 animate-spin" /> Saving...
            </span>
          )}
        </div>
      </div>
      <textarea
        className="flex-1 w-full p-4 text-sm border-0 resize-none focus:outline-none focus:ring-0"
        placeholder="Write your meeting notes in markdown..."
        value={markdown}
        onChange={(e) => handleChange(e.target.value)}
      />
    </div>
  );
}

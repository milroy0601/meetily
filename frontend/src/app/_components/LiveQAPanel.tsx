'use client';

import { useState, useRef, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MessageCircle, Send, Loader2, Bot, User, Plus, Trash2 } from 'lucide-react';
import { useTranscripts } from '@/contexts/TranscriptContext';

interface ChatMessage {
  id?: number;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
}

interface ChatSession {
  session_id: string;
  message_count: number;
}

/**
 * LiveQAPanel - Ask questions about the live meeting transcript.
 * Messages persist to SQLite and are grouped by session.
 */
export function LiveQAPanel() {
  const { transcripts, currentMeetingId } = useTranscripts();
  const [question, setQuestion] = useState('');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string>('');
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const loadedRef = useRef(false);

  const newSessionId = () => `live-${Date.now()}`;

  // Initialize session on mount — load existing sessions if any
  useEffect(() => {
    if (loadedRef.current || !currentMeetingId) return;
    loadedRef.current = true;

    const init = async () => {
      try {
        const sess = await invoke<ChatSession[]>('get_chat_sessions', { meetingId: currentMeetingId });
        if (sess.length > 0) {
          setSessions(sess);
          setSessionId(sess[0].session_id);
          const msgs = await invoke<ChatMessage[]>('get_chat_session_messages', {
            meetingId: currentMeetingId,
            sessionId: sess[0].session_id,
          });
          setMessages(msgs);
        } else {
          setSessionId(newSessionId());
        }
      } catch {
        setSessionId(newSessionId());
      }
    };
    init();
  }, [currentMeetingId]);

  // Save message to DB
  const persistMessage = useCallback(async (msg: ChatMessage) => {
    if (!sessionId) return;
    try {
      await invoke('save_chat_message', {
        meetingId: currentMeetingId || '',
        role: msg.role,
        content: msg.content,
        sessionId,
      });
      refreshSessions();
    } catch (e) { console.warn('Persist failed:', e); }
  }, [sessionId, currentMeetingId]);

  const refreshSessions = async () => {
    if (!currentMeetingId) return;
    try {
      const sess = await invoke<ChatSession[]>('get_chat_sessions', { meetingId: currentMeetingId });
      setSessions(sess);
    } catch { /* ignore */ }
  };

  const startNewChat = () => {
    setMessages([]);
    setError(null);
    setSessionId(newSessionId());
  };

  const switchSession = async (sid: string) => {
    setSessionId(sid);
    try {
      const msgs = await invoke<ChatMessage[]>('get_chat_session_messages', {
        meetingId: currentMeetingId,
        sessionId: sid,
      });
      setMessages(msgs);
    } catch { /* ignore */ }
  };

  const deleteSession = async (sid: string) => {
    try {
      await invoke('delete_chat_session', { meetingId: currentMeetingId, sessionId: sid });
      if (sid === sessionId) {
        setMessages([]);
        setSessionId(newSessionId());
      }
      refreshSessions();
    } catch (e) { console.error(e); }
  };

  // Build rolling 10-minute transcript buffer
  const getRecentTranscript = useCallback((): string => {
    const TEN_MINUTES_MS = 10 * 60 * 1000;
    const now = Date.now();
    const recent = transcripts
      .filter((t) => {
        if (t.audio_start_time != null) {
          const absoluteTime = now - t.audio_start_time * 1000;
          return now - absoluteTime <= TEN_MINUTES_MS;
        }
        return true;
      })
      .slice(-100);

    return recent
      .map((t) => {
        const ts = t.timestamp ? new Date(t.timestamp).toLocaleTimeString() : '';
        return `[${ts}] ${t.text}`;
      })
      .join('\n');
  }, [transcripts]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const q = question.trim();
    if (!q || isLoading) return;

    const userMsg: ChatMessage = { role: 'user', content: q, timestamp: Date.now() };
    setMessages((prev) => [...prev, userMsg]);
    setQuestion('');
    setIsLoading(true);
    setError(null);
    persistMessage(userMsg);

    try {
      const context = getRecentTranscript();
      if (!context.trim()) {
        setError('No transcript available yet.');
        setIsLoading(false);
        return;
      }

      const response = await invoke<string>('ask_llama', {
        mode: 'builtin-ai',
        context,
        question: q,
      });

      const aiMsg: ChatMessage = { role: 'assistant', content: response, timestamp: Date.now() };
      setMessages((prev) => [...prev, aiMsg]);
      persistMessage(aiMsg);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  return (
    <div className="flex flex-col h-full">
      <div className="sticky top-0 z-10 bg-card px-4 py-3 border-b">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <MessageCircle className="w-4 h-4 text-blue-600" />
            <h3 className="text-sm font-semibold text-gray-700">Live Q&A</h3>
          </div>
          {messages.length > 0 && (
            <button
              onClick={startNewChat}
              className="flex items-center space-x-1 text-xs text-blue-600 hover:text-blue-700"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>New Chat</span>
            </button>
          )}
        </div>
        <p className="text-xs text-gray-400 mt-1">Last 10 min of transcript</p>
        {/* Session tabs */}
        {sessions.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-1">
            {sessions.map((s) => (
              <div key={s.session_id} className="flex items-center">
                <button
                  onClick={() => switchSession(s.session_id)}
                  className={`text-xs px-2 py-0.5 rounded ${
                    s.session_id === sessionId
                      ? 'bg-accent text-accent-foreground'
                      : 'bg-muted text-muted-foreground hover:bg-secondary'
                  }`}
                >
                  Chat ({s.message_count})
                </button>
                <button
                  onClick={() => deleteSession(s.session_id)}
                  className="text-gray-400 hover:text-red-500 ml-0.5"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {messages.length === 0 && !isLoading && (
          <div className="text-center text-gray-400 text-sm py-8">
            <Bot className="w-8 h-8 mx-auto mb-2 opacity-50" />
            <p>Ask a question about the ongoing meeting</p>
          </div>
        )}

        {messages.map((msg, idx) => (
          <div key={idx} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
            <div className={`max-w-[85%] rounded-lg p-2.5 text-sm ${msg.role === 'user' ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-800'}`}>
              <div className="flex items-center space-x-1 mb-0.5">
                {msg.role === 'user' ? <User className="w-3 h-3" /> : <Bot className="w-3 h-3" />}
                <span className="text-xs opacity-70">{msg.role === 'user' ? 'You' : 'AI'}</span>
              </div>
              <div className="whitespace-pre-wrap">{msg.content}</div>
            </div>
          </div>
        ))}

        {isLoading && (
          <div className="flex justify-start">
            <div className="bg-gray-100 rounded-lg p-3">
              <Loader2 className="w-4 h-4 animate-spin text-gray-500" />
            </div>
          </div>
        )}

        {error && <div className="text-center text-red-500 text-xs p-2 bg-red-50 rounded">{error}</div>}
        <div ref={messagesEndRef} />
      </div>

      <div className="border-t border-gray-200 p-3">
        <form onSubmit={handleSubmit} className="flex space-x-2">
          <input
            type="text"
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            placeholder="Ask about the meeting..."
            className="flex-1 px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-400"
            disabled={isLoading}
          />
          <button type="submit" disabled={isLoading || !question.trim()}
            className="px-3 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50">
            {isLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
          </button>
        </form>
      </div>
    </div>
  );
}

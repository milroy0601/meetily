'use client';

import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MessageCircle, Send, Loader2, Bot, User, FileText, Plus, Trash2 } from 'lucide-react';
import { Transcript } from '@/types';

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

interface PostMeetingQAProps {
  meetingId: string;
  transcripts: Transcript[];
}

export function PostMeetingQA({ meetingId, transcripts }: PostMeetingQAProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [question, setQuestion] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [contextReady, setContextReady] = useState(false);
  const [sessionId, setSessionId] = useState<string>('');
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const fullContextRef = useRef<string>('');
  const hasBuiltRef = useRef(false);

  const newSessionId = () => `chat-${Date.now()}`;

  // Initialize: build context, load sessions and latest chat
  useEffect(() => {
    if (hasBuiltRef.current) return;
    hasBuiltRef.current = true;

    const init = async () => {
      try {
        // Build context
        const transcriptText = transcripts
          .map((t) => {
            const ts = t.timestamp ? new Date(t.timestamp).toLocaleTimeString() : '';
            return `[${ts}] ${t.text}`;
          })
          .join('\n');

        let notesText = '';
        try {
          const note = await invoke<string | null>('get_meeting_markdown_note', { meetingId });
          if (note) notesText = '\n\nMEETING NOTES:\n' + note;
        } catch { /* ignore */ }

        fullContextRef.current = transcriptText + notesText;
        setContextReady(true);

        // Load sessions
        const sess = await invoke<ChatSession[]>('get_chat_sessions', { meetingId });
        setSessions(sess);

        // Load latest session messages
        if (sess.length > 0) {
          const msgs = await invoke<ChatMessage[]>('get_chat_session_messages', {
            meetingId,
            sessionId: sess[0].session_id,
          });
          setMessages(msgs);
          setSessionId(sess[0].session_id);
        } else {
          setSessionId(newSessionId());
        }
      } catch (e) {
        console.error('Init failed:', e);
        setError('Failed to load meeting data');
      }
    };
    init();
  }, [meetingId, transcripts]);

  // Persist each message
  const persistMessage = useCallback(async (msg: ChatMessage) => {
    try {
      await invoke('save_chat_message', {
        meetingId,
        role: msg.role,
        content: msg.content,
        sessionId,
      });
      // Refresh sessions list in background
      refreshSessions();
    } catch (e) { console.warn('Persist failed:', e); }
  }, [meetingId, sessionId]);

  const refreshSessions = async () => {
    try {
      const sess = await invoke<ChatSession[]>('get_chat_sessions', { meetingId });
      setSessions(sess);
    } catch { /* ignore */ }
  };

  // Submit question
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const q = question.trim();
    if (!q || isLoading || !contextReady) return;

    const userMsg: ChatMessage = { role: 'user', content: q, timestamp: Date.now() };
    setMessages((prev) => [...prev, userMsg]);
    setQuestion('');
    setIsLoading(true);
    setError(null);
    persistMessage(userMsg);

    try {
      const response = await invoke<string>('ask_llama', {
        mode: 'builtin-ai',
        context: fullContextRef.current,
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

  // Start new chat
  const startNewChat = () => {
    setMessages([]);
    setError(null);
    setSessionId(newSessionId());
  };

  // Delete session
  const deleteSession = async (sid: string) => {
    try {
      await invoke('delete_chat_session', { meetingId, sessionId: sid });
      if (sid === sessionId) {
        setMessages([]);
        setSessionId(newSessionId());
      }
      refreshSessions();
    } catch (e) { console.error(e); }
  };

  // Switch session
  const switchSession = async (sid: string) => {
    setSessionId(sid);
    try {
      const msgs = await invoke<ChatMessage[]>('get_chat_session_messages', { meetingId, sessionId: sid });
      setMessages(msgs);
    } catch (e) { console.error(e); }
  };

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  return (
    <div className="flex flex-col h-full bg-white">
      <div className="sticky top-0 z-10 bg-white p-4 border-b border-gray-200 space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <MessageCircle className="w-4 h-4 text-blue-600" />
            <h3 className="text-sm font-semibold text-gray-700">Meeting Q&A</h3>
          </div>
          <div className="flex items-center space-x-2">
            {contextReady && (
              <span className="text-xs text-green-600 flex items-center gap-1">
                <FileText className="w-3 h-3" /> Ready
              </span>
            )}
            <button onClick={startNewChat}
              className="flex items-center space-x-1 text-xs text-blue-600 hover:text-blue-700">
              <Plus className="w-3.5 h-3.5" /> New Chat
            </button>
          </div>
        </div>

        {/* Session history tabs */}
        {sessions.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {sessions.map((s) => (
              <div key={s.session_id} className="flex items-center">
                <button
                  onClick={() => switchSession(s.session_id)}
                  className={`text-xs px-2 py-1 rounded ${
                    s.session_id === sessionId
                      ? 'bg-accent text-accent-foreground'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
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
            <p>Ask anything about this meeting</p>
          </div>
        )}

        {messages.map((msg, idx) => (
          <div key={idx} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
            <div className={`max-w-[85%] rounded-lg p-3 text-sm ${msg.role === 'user' ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-800'}`}>
              <div className="flex items-center space-x-1 mb-1">
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
            placeholder={contextReady ? 'Ask about this meeting...' : 'Loading...'}
            className="flex-1 px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-400"
            disabled={isLoading || !contextReady}
          />
          <button type="submit" disabled={isLoading || !question.trim() || !contextReady}
            className="px-3 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50">
            {isLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
          </button>
        </form>
      </div>
    </div>
  );
}

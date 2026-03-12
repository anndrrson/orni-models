"use client";

import { useState, useEffect } from "react";
import { getChatSessions, type SessionSummary } from "@/lib/api";
import { History, ChevronLeft, ChevronRight, MessageSquare } from "lucide-react";

interface ConversationSidebarProps {
  slug: string;
  onSelectSession: (sessionId: string) => void;
}

export default function ConversationSidebar({
  slug,
  onSelectSession,
}: ConversationSidebarProps) {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [collapsed, setCollapsed] = useState(false);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    getChatSessions()
      .then((all) => {
        setSessions(all.filter((s) => s.model_slug === slug));
      })
      .catch(() => setSessions([]))
      .finally(() => setLoading(false));
  }, [slug]);

  function formatTimestamp(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    }
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString([], { month: "short", day: "numeric" });
  }

  function truncate(text: string, maxLen: number): string {
    if (text.length <= maxLen) return text;
    return text.slice(0, maxLen).trimEnd() + "...";
  }

  function handleSelect(sessionId: string) {
    setSelectedId(sessionId);
    onSelectSession(sessionId);
  }

  if (collapsed) {
    return (
      <div className="flex flex-col items-center rounded-xl border border-gray-800 bg-gray-900 py-3 px-1.5">
        <button
          onClick={() => setCollapsed(false)}
          className="flex h-8 w-8 items-center justify-center rounded-lg text-gray-400 transition hover:bg-gray-800 hover:text-white"
          title="Show history"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
        <History className="mt-2 h-4 w-4 text-gray-600" />
      </div>
    );
  }

  return (
    <div className="flex w-64 flex-col rounded-xl border border-gray-800 bg-gray-900">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-gray-800 px-4 py-3">
        <div className="flex items-center gap-2">
          <History className="h-4 w-4 text-gray-400" />
          <span className="text-sm font-medium text-gray-300">History</span>
        </div>
        <button
          onClick={() => setCollapsed(true)}
          className="flex h-6 w-6 items-center justify-center rounded-md text-gray-500 transition hover:bg-gray-800 hover:text-gray-300"
          title="Collapse"
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Session list */}
      <div className="flex-1 overflow-y-auto">
        {loading && (
          <div className="flex items-center justify-center py-8">
            <div className="h-5 w-5 animate-spin rounded-full border-2 border-gray-700 border-t-coral-400" />
          </div>
        )}

        {!loading && sessions.length === 0 && (
          <div className="px-4 py-8 text-center">
            <MessageSquare className="mx-auto mb-2 h-5 w-5 text-gray-700" />
            <p className="text-xs text-gray-600">No conversations yet</p>
          </div>
        )}

        {!loading &&
          sessions.map((session) => (
            <button
              key={session.id}
              onClick={() => handleSelect(session.id)}
              className={`w-full border-b border-gray-800/50 px-4 py-3 text-left transition hover:bg-gray-800/60 ${
                selectedId === session.id
                  ? "bg-gray-800 border-l-2 border-l-coral-500"
                  : ""
              }`}
            >
              <p className="text-sm text-gray-300 leading-snug">
                {session.last_message
                  ? truncate(session.last_message, 60)
                  : "New conversation"}
              </p>
              <div className="mt-1.5 flex items-center justify-between text-xs text-gray-500">
                <span>{formatTimestamp(session.updated_at)}</span>
                <span className="flex items-center gap-1">
                  <MessageSquare className="h-3 w-3" />
                  {session.message_count}
                </span>
              </div>
            </button>
          ))}
      </div>
    </div>
  );
}

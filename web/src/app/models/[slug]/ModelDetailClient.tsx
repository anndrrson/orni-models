"use client";

import { useEffect, useState, useCallback } from "react";
import {
  getModel,
  getBalance,
  getModelUsage,
  type Model,
  type UsageInfo,
} from "@/lib/api";
import { useAuth } from "@/lib/wallet-provider";
import ChatInterface from "@/components/ChatInterface";
import ConversationSidebar from "@/components/ConversationSidebar";
import StarRating from "@/components/StarRating";
import ReviewSection from "@/components/ReviewSection";
import {
  MessageSquare,
  Clock,
  DollarSign,
  Code2,
  Copy,
  Check,
  ChevronDown,
  ChevronUp,
  Zap,
} from "lucide-react";
import Link from "next/link";

const API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api";

export default function ModelDetailClient({ slug }: { slug: string }) {
  const { authenticated } = useAuth();
  const [model, setModel] = useState<Model | null>(null);
  const [balance, setBalance] = useState<number | null>(null);
  const [usage, setUsage] = useState<UsageInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [apiPanelOpen, setApiPanelOpen] = useState(false);
  const [copiedSnippet, setCopiedSnippet] = useState<string | null>(null);

  const fetchBalance = useCallback(() => {
    if (authenticated) {
      getBalance()
        .then((b) => setBalance(b.balance))
        .catch(() => setBalance(null));
    }
  }, [authenticated]);

  const fetchUsage = useCallback(() => {
    if (authenticated) {
      getModelUsage(slug)
        .then(setUsage)
        .catch(() => setUsage(null));
    }
  }, [authenticated, slug]);

  useEffect(() => {
    getModel(slug)
      .then(setModel)
      .catch(() => setError("Model not found"))
      .finally(() => setLoading(false));
  }, [slug]);

  useEffect(() => {
    fetchBalance();
  }, [fetchBalance]);

  useEffect(() => {
    fetchUsage();
  }, [fetchUsage]);

  const handleBalanceUpdate = useCallback(() => {
    fetchBalance();
    fetchUsage();
  }, [fetchBalance, fetchUsage]);

  const handleSelectSession = useCallback((sessionId: string) => {
    setActiveSessionId(sessionId);
  }, []);

  const handleSessionChange = useCallback((sessionId: string) => {
    setActiveSessionId(sessionId);
  }, []);

  const copyToClipboard = useCallback((text: string, key: string) => {
    navigator.clipboard.writeText(text).then(() => {
      setCopiedSnippet(key);
      setTimeout(() => setCopiedSnippet(null), 2000);
    });
  }, []);

  // ── Loading state ──

  if (loading) {
    return (
      <div className="mx-auto max-w-7xl px-4 py-8">
        <div className="grid gap-6 lg:grid-cols-[340px_1fr]">
          <div className="space-y-4">
            <div className="h-72 animate-pulse rounded-xl bg-gray-900" />
            <div className="h-24 animate-pulse rounded-xl bg-gray-900" />
          </div>
          <div className="h-[calc(100vh-8rem)] animate-pulse rounded-xl bg-gray-900" />
        </div>
      </div>
    );
  }

  // ── Error state ──

  if (error || !model) {
    return (
      <div className="mx-auto max-w-7xl px-4 py-16 text-center">
        <h1 className="mb-4 text-2xl font-bold">Model not found</h1>
        <Link href="/models" className="text-coral-400 hover:text-coral-300">
          Back to browse
        </Link>
      </div>
    );
  }

  // ── Helpers ──

  const priceDisplay =
    model.free_queries_per_day && model.free_queries_per_day > 0
      ? `${model.free_queries_per_day} free/day`
      : `$${model.price_per_query.toFixed(2)}/query`;

  const endpointUrl = `${API_BASE}/chat/${slug}/message`;

  const curlSnippet = `curl -X POST "${endpointUrl}" \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"message": "Hello!"}'`;

  const pythonSnippet = `import requests

response = requests.post(
    "${endpointUrl}",
    headers={
        "Authorization": "Bearer YOUR_API_KEY",
        "Content-Type": "application/json",
    },
    json={"message": "Hello!"},
    stream=True,
)

for line in response.iter_lines():
    if line:
        print(line.decode())`;

  const jsSnippet = `const response = await fetch("${endpointUrl}", {
  method: "POST",
  headers: {
    "Authorization": "Bearer YOUR_API_KEY",
    "Content-Type": "application/json",
  },
  body: JSON.stringify({ message: "Hello!" }),
});

const reader = response.body.getReader();
const decoder = new TextDecoder();
while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  console.log(decoder.decode(value));
}`;

  // ── Render ──

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      <div className="grid gap-6 lg:grid-cols-[340px_1fr]">
        {/* ── Left column: Model info + Balance + Usage ── */}
        <div className="space-y-4">
          {/* Model info card */}
          <div className="rounded-xl border border-gray-800 bg-gray-900 p-6">
            <div className="mb-4 flex items-center gap-3">
              <div className="flex h-12 w-12 items-center justify-center rounded-full bg-gradient-to-br from-coral-500 to-purple-500 text-lg font-bold text-white">
                {model.creator_name?.[0]?.toUpperCase() || "?"}
              </div>
              <div>
                <h1 className="text-xl font-bold">{model.name}</h1>
                <p className="text-sm text-gray-400">
                  {model.creator_slug ? (
                    <Link
                      href={`/creators/${model.creator_slug}`}
                      className="hover:text-coral-400 transition"
                    >
                      {model.creator_name}
                    </Link>
                  ) : (
                    model.creator_name
                  )}
                </p>
              </div>
            </div>

            <div className="mb-4 flex flex-wrap items-center gap-2">
              {model.category && (
                <span className="rounded-full bg-coral-500/10 px-3 py-1 text-xs font-medium text-coral-400">
                  {model.category}
                </span>
              )}
              {model.free_queries_per_day && model.free_queries_per_day > 0 && (
                <span className="rounded-full bg-green-500/10 px-3 py-1 text-xs font-medium text-green-400">
                  {model.free_queries_per_day} free/day
                </span>
              )}
            </div>

            {/* Rating */}
            {model.review_count > 0 && (
              <div className="mb-4 flex items-center gap-2">
                <StarRating rating={model.avg_rating} size="sm" />
                <span className="text-sm text-gray-400">
                  {model.avg_rating.toFixed(1)}
                </span>
                <span className="text-xs text-gray-600">
                  ({model.review_count}{" "}
                  {model.review_count === 1 ? "review" : "reviews"})
                </span>
              </div>
            )}

            <p className="mb-6 text-sm text-gray-400">{model.description}</p>

            <div className="grid grid-cols-2 gap-3">
              <div className="rounded-lg bg-gray-800 p-3 text-center">
                <DollarSign className="mx-auto mb-1 h-4 w-4 text-coral-400" />
                <p className="text-lg font-bold">{priceDisplay}</p>
                <p className="text-xs text-gray-500">pricing</p>
              </div>
              <div className="rounded-lg bg-gray-800 p-3 text-center">
                <MessageSquare className="mx-auto mb-1 h-4 w-4 text-coral-400" />
                <p className="text-lg font-bold">
                  {model.total_queries.toLocaleString()}
                </p>
                <p className="text-xs text-gray-500">queries</p>
              </div>
              <div className="rounded-lg bg-gray-800 p-3 text-center">
                <Clock className="mx-auto mb-1 h-4 w-4 text-gray-400" />
                <p className="text-sm font-medium">
                  {new Date(model.created_at).toLocaleDateString()}
                </p>
                <p className="text-xs text-gray-500">created</p>
              </div>
              {model.tags && model.tags.length > 0 && (
                <div className="rounded-lg bg-gray-800 p-3 text-center">
                  <p className="text-xs text-gray-400 leading-relaxed">
                    {model.tags.slice(0, 3).join(", ")}
                  </p>
                  <p className="mt-1 text-xs text-gray-500">tags</p>
                </div>
              )}
            </div>
          </div>

          {/* Balance card */}
          {authenticated && (
            <div className="rounded-xl border border-gray-800 bg-gray-900 p-4 text-center">
              <p className="text-sm text-gray-400">Your Balance</p>
              <p className="text-2xl font-bold text-white">
                ${balance !== null ? balance.toFixed(2) : "--"}
              </p>
              <Link
                href="/account"
                className="mt-2 inline-block text-xs text-coral-400 hover:text-coral-300"
              >
                Add funds
              </Link>
            </div>
          )}

          {/* Free tier usage */}
          {authenticated && usage && usage.is_free && (
            <div className="rounded-xl border border-gray-800 bg-gray-900 p-4">
              <div className="mb-2 flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Zap className="h-4 w-4 text-green-400" />
                  <span className="text-sm font-medium text-gray-300">
                    Free Usage
                  </span>
                </div>
                <span className="text-xs text-gray-500">
                  {usage.used}/{usage.limit} today
                </span>
              </div>
              <div className="h-2 rounded-full bg-gray-800">
                <div
                  className={`h-2 rounded-full transition-all ${
                    usage.used >= usage.limit
                      ? "bg-red-500"
                      : usage.used >= usage.limit * 0.8
                        ? "bg-yellow-500"
                        : "bg-green-500"
                  }`}
                  style={{
                    width: `${Math.min((usage.used / usage.limit) * 100, 100)}%`,
                  }}
                />
              </div>
              {usage.used >= usage.limit && (
                <p className="mt-2 text-xs text-red-400">
                  Free tier limit reached. Add funds to continue.
                </p>
              )}
            </div>
          )}

          {/* Conversation history sidebar (desktop) */}
          {authenticated && (
            <div className="hidden lg:block">
              <ConversationSidebar
                slug={slug}
                onSelectSession={handleSelectSession}
              />
            </div>
          )}
        </div>

        {/* ── Right column: Chat + extras ── */}
        <div className="space-y-4">
          {/* Chat area */}
          <div className="flex h-[calc(100vh-8rem)] flex-col rounded-xl border border-gray-800 bg-gray-900">
            {authenticated ? (
              <ChatInterface
                slug={slug}
                pricePerQuery={model.price_per_query}
                balance={balance}
                onBalanceUpdate={handleBalanceUpdate}
                initialSessionId={activeSessionId || undefined}
                onSessionChange={handleSessionChange}
              />
            ) : (
              <div className="flex h-full items-center justify-center">
                <div className="text-center">
                  <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-gray-800">
                    <MessageSquare className="h-7 w-7 text-gray-600" />
                  </div>
                  <p className="mb-2 text-gray-400">
                    Connect your wallet to start chatting
                  </p>
                  <p className="text-xs text-gray-600">
                    Each message costs {priceDisplay}
                  </p>
                </div>
              </div>
            )}
          </div>

          {/* Conversation history (mobile) */}
          {authenticated && (
            <div className="lg:hidden">
              <ConversationSidebar
                slug={slug}
                onSelectSession={handleSelectSession}
              />
            </div>
          )}

          {/* API Access panel */}
          <div className="rounded-xl border border-gray-800 bg-gray-900">
            <button
              onClick={() => setApiPanelOpen(!apiPanelOpen)}
              className="flex w-full items-center justify-between p-4 text-left"
            >
              <div className="flex items-center gap-2">
                <Code2 className="h-4 w-4 text-coral-400" />
                <h3 className="font-semibold text-white">API Access</h3>
              </div>
              {apiPanelOpen ? (
                <ChevronUp className="h-4 w-4 text-gray-400" />
              ) : (
                <ChevronDown className="h-4 w-4 text-gray-400" />
              )}
            </button>

            {apiPanelOpen && (
              <div className="border-t border-gray-800 p-4 space-y-4">
                {/* Endpoint */}
                <div>
                  <p className="mb-1.5 text-xs font-medium text-gray-400">
                    Endpoint
                  </p>
                  <div className="flex items-center gap-2 rounded-lg bg-gray-800 px-3 py-2">
                    <code className="flex-1 truncate text-sm text-coral-300 font-mono">
                      POST {endpointUrl}
                    </code>
                    <button
                      onClick={() => copyToClipboard(endpointUrl, "endpoint")}
                      className="shrink-0 rounded p-1 text-gray-500 transition hover:bg-gray-700 hover:text-gray-300"
                    >
                      {copiedSnippet === "endpoint" ? (
                        <Check className="h-3.5 w-3.5 text-green-400" />
                      ) : (
                        <Copy className="h-3.5 w-3.5" />
                      )}
                    </button>
                  </div>
                </div>

                {/* Code snippets */}
                <CodeSnippet
                  label="cURL"
                  code={curlSnippet}
                  snippetKey="curl"
                  copiedSnippet={copiedSnippet}
                  onCopy={copyToClipboard}
                />
                <CodeSnippet
                  label="Python"
                  code={pythonSnippet}
                  snippetKey="python"
                  copiedSnippet={copiedSnippet}
                  onCopy={copyToClipboard}
                />
                <CodeSnippet
                  label="JavaScript"
                  code={jsSnippet}
                  snippetKey="js"
                  copiedSnippet={copiedSnippet}
                  onCopy={copyToClipboard}
                />

                <p className="text-xs text-gray-600">
                  Create an API key from your{" "}
                  <Link
                    href="/account"
                    className="text-coral-400 hover:text-coral-300"
                  >
                    account page
                  </Link>{" "}
                  to use the API.
                </p>
              </div>
            )}
          </div>

          {/* Reviews section */}
          <ReviewSection slug={slug} authenticated={authenticated} />
        </div>
      </div>
    </div>
  );
}

// ── Code snippet sub-component ──

function CodeSnippet({
  label,
  code,
  snippetKey,
  copiedSnippet,
  onCopy,
}: {
  label: string;
  code: string;
  snippetKey: string;
  copiedSnippet: string | null;
  onCopy: (text: string, key: string) => void;
}) {
  return (
    <div>
      <div className="mb-1.5 flex items-center justify-between">
        <p className="text-xs font-medium text-gray-400">{label}</p>
        <button
          onClick={() => onCopy(code, snippetKey)}
          className="flex items-center gap-1 rounded px-2 py-0.5 text-xs text-gray-500 transition hover:bg-gray-800 hover:text-gray-300"
        >
          {copiedSnippet === snippetKey ? (
            <>
              <Check className="h-3 w-3 text-green-400" />
              Copied
            </>
          ) : (
            <>
              <Copy className="h-3 w-3" />
              Copy
            </>
          )}
        </button>
      </div>
      <pre className="overflow-x-auto rounded-lg bg-gray-800 p-3 text-xs text-gray-300 font-mono leading-relaxed">
        {code}
      </pre>
    </div>
  );
}

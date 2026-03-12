"use client";

import { useState, useEffect, useCallback, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import { getModels, type Model } from "@/lib/api";
import CompareView from "@/components/CompareView";
import { Send, Loader2, ArrowLeftRight, ThumbsUp, RotateCcw } from "lucide-react";

function ComparePageInner() {
  const searchParams = useSearchParams();
  const router = useRouter();

  const [models, setModels] = useState<Model[]>([]);
  const [loading, setLoading] = useState(true);

  const [slugA, setSlugA] = useState(searchParams.get("a") || "");
  const [slugB, setSlugB] = useState(searchParams.get("b") || "");

  const [input, setInput] = useState("");
  const [activeMessage, setActiveMessage] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [bothDone, setBothDone] = useState(false);
  const [vote, setVote] = useState<"a" | "b" | null>(null);

  // Fetch all available models
  useEffect(() => {
    (async () => {
      try {
        const res = await getModels({ limit: 100 });
        setModels(res.models);
      } catch {
        setModels([]);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  // Sync URL params when selectors change
  useEffect(() => {
    const params = new URLSearchParams();
    if (slugA) params.set("a", slugA);
    if (slugB) params.set("b", slugB);
    const qs = params.toString();
    router.replace(`/compare${qs ? `?${qs}` : ""}`, { scroll: false });
  }, [slugA, slugB, router]);

  const modelA = models.find((m) => m.slug === slugA);
  const modelB = models.find((m) => m.slug === slugB);

  const canCompare = slugA && slugB && slugA !== slugB;

  const handleSend = () => {
    const text = input.trim();
    if (!text || !canCompare || isStreaming) return;
    setInput("");
    setActiveMessage(text);
    setIsStreaming(true);
    setBothDone(false);
    setVote(null);
  };

  const handleComplete = useCallback(() => {
    setIsStreaming(false);
    setBothDone(true);
  }, []);

  const handleSwap = () => {
    const tmpA = slugA;
    setSlugA(slugB);
    setSlugB(tmpA);
  };

  const handleReset = () => {
    setActiveMessage("");
    setIsStreaming(false);
    setBothDone(false);
    setVote(null);
  };

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold">Compare Models</h1>
        <p className="mt-2 text-sm text-gray-400">
          Send the same question to two models and see how they stack up side by side.
        </p>
      </div>

      {/* Model Selectors */}
      <div className="mb-6 flex flex-col items-center gap-3 md:flex-row">
        <div className="flex-1 w-full">
          <label className="mb-1.5 block text-xs font-medium text-gray-400">Model A</label>
          <select
            value={slugA}
            onChange={(e) => setSlugA(e.target.value)}
            disabled={isStreaming}
            className="w-full rounded-xl border border-gray-700 bg-gray-900 px-4 py-2.5 text-sm text-gray-200 outline-none transition focus:border-coral-500 disabled:opacity-50"
          >
            <option value="">Select a model...</option>
            {models
              .filter((m) => m.slug !== slugB)
              .map((m) => (
                <option key={m.id} value={m.slug}>
                  {m.name} {m.creator_name ? `by ${m.creator_name}` : ""}
                </option>
              ))}
          </select>
        </div>

        <button
          onClick={handleSwap}
          disabled={isStreaming || !slugA || !slugB}
          className="mt-5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-gray-700 bg-gray-800 text-gray-400 transition hover:border-coral-500 hover:text-coral-400 disabled:opacity-30 disabled:hover:border-gray-700 disabled:hover:text-gray-400"
          title="Swap models"
        >
          <ArrowLeftRight className="h-4 w-4" />
        </button>

        <div className="flex-1 w-full">
          <label className="mb-1.5 block text-xs font-medium text-gray-400">Model B</label>
          <select
            value={slugB}
            onChange={(e) => setSlugB(e.target.value)}
            disabled={isStreaming}
            className="w-full rounded-xl border border-gray-700 bg-gray-900 px-4 py-2.5 text-sm text-gray-200 outline-none transition focus:border-coral-500 disabled:opacity-50"
          >
            <option value="">Select a model...</option>
            {models
              .filter((m) => m.slug !== slugA)
              .map((m) => (
                <option key={m.id} value={m.slug}>
                  {m.name} {m.creator_name ? `by ${m.creator_name}` : ""}
                </option>
              ))}
          </select>
        </div>
      </div>

      {/* Loading state */}
      {loading && (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-6 w-6 animate-spin text-gray-500" />
        </div>
      )}

      {/* Compare View */}
      {activeMessage && modelA && modelB && (
        <div className="mb-6">
          <div className="mb-3 rounded-xl bg-gray-900 border border-gray-800 px-4 py-3">
            <p className="text-xs font-medium text-gray-500 mb-1">Your question</p>
            <p className="text-sm text-gray-200">{activeMessage}</p>
          </div>

          <CompareView
            modelA={{ slug: modelA.slug, name: modelA.name }}
            modelB={{ slug: modelB.slug, name: modelB.name }}
            message={activeMessage}
            onComplete={handleComplete}
          />

          {/* Vote Buttons */}
          {bothDone && (
            <div className="mt-4 flex flex-col items-center gap-3">
              <p className="text-sm font-medium text-gray-400">Which response was better?</p>
              <div className="flex items-center gap-3">
                <button
                  onClick={() => setVote("a")}
                  className={`flex items-center gap-2 rounded-xl px-5 py-2.5 text-sm font-medium transition ${
                    vote === "a"
                      ? "bg-coral-500 text-white"
                      : "border border-gray-700 bg-gray-800 text-gray-300 hover:border-coral-500 hover:text-coral-400"
                  }`}
                >
                  <ThumbsUp className="h-4 w-4" />
                  {modelA.name}
                </button>
                <button
                  onClick={() => setVote("b")}
                  className={`flex items-center gap-2 rounded-xl px-5 py-2.5 text-sm font-medium transition ${
                    vote === "b"
                      ? "bg-purple-500 text-white"
                      : "border border-gray-700 bg-gray-800 text-gray-300 hover:border-purple-500 hover:text-purple-400"
                  }`}
                >
                  <ThumbsUp className="h-4 w-4" />
                  {modelB.name}
                </button>
                <button
                  onClick={handleReset}
                  className="flex items-center gap-2 rounded-xl border border-gray-700 bg-gray-800 px-4 py-2.5 text-sm text-gray-400 transition hover:border-gray-600 hover:text-gray-300"
                >
                  <RotateCcw className="h-4 w-4" />
                  Try again
                </button>
              </div>
              {vote && (
                <p className="text-xs text-gray-500 mt-1">
                  You voted for <span className={vote === "a" ? "text-coral-400" : "text-purple-400"}>{vote === "a" ? modelA.name : modelB.name}</span>. Thanks for the feedback!
                </p>
              )}
            </div>
          )}
        </div>
      )}

      {/* Empty state */}
      {!activeMessage && !loading && (
        <div className="mb-6 flex items-center justify-center rounded-xl border border-gray-800 bg-gray-900/50 py-20">
          <div className="text-center">
            <ArrowLeftRight className="mx-auto mb-3 h-8 w-8 text-gray-600" />
            <p className="text-sm text-gray-500">
              {canCompare
                ? "Type a message below to compare both models"
                : "Select two different models to get started"}
            </p>
          </div>
        </div>
      )}

      {/* Input Bar */}
      <div className="rounded-xl border border-gray-800 bg-gray-900 p-4">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && handleSend()}
            placeholder={
              canCompare
                ? "Ask both models the same question..."
                : "Select two models first..."
            }
            disabled={isStreaming || !canCompare}
            className="flex-1 rounded-xl border border-gray-700 bg-gray-800 px-4 py-2.5 text-sm text-white placeholder-gray-500 outline-none transition focus:border-coral-500 disabled:opacity-50"
          />
          <button
            onClick={handleSend}
            disabled={isStreaming || !input.trim() || !canCompare}
            className="flex h-10 w-10 items-center justify-center rounded-xl bg-coral-600 text-white transition hover:bg-coral-500 disabled:opacity-50 disabled:hover:bg-coral-600"
          >
            {isStreaming ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
          </button>
        </div>
      </div>
    </div>
  );
}

export default function ComparePage() {
  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-32">
          <Loader2 className="h-6 w-6 animate-spin text-gray-500" />
        </div>
      }
    >
      <ComparePageInner />
    </Suspense>
  );
}

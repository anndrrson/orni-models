"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { sendMessage } from "@/lib/api";
import { Loader2 } from "lucide-react";

interface ModelInfo {
  slug: string;
  name: string;
}

interface CompareViewProps {
  modelA: ModelInfo;
  modelB: ModelInfo;
  message: string;
  onComplete: () => void;
}

export default function CompareView({
  modelA,
  modelB,
  message,
  onComplete,
}: CompareViewProps) {
  const [responseA, setResponseA] = useState("");
  const [responseB, setResponseB] = useState("");
  const [streamingA, setStreamingA] = useState(false);
  const [streamingB, setStreamingB] = useState(false);
  const [errorA, setErrorA] = useState<string | null>(null);
  const [errorB, setErrorB] = useState<string | null>(null);

  const colARef = useRef<HTMLDivElement>(null);
  const colBRef = useRef<HTMLDivElement>(null);

  const doneA = useRef(false);
  const doneB = useRef(false);

  const scrollToBottom = useCallback((ref: React.RefObject<HTMLDivElement | null>) => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight, behavior: "smooth" });
  }, []);

  useEffect(() => {
    if (!message) return;

    setResponseA("");
    setResponseB("");
    setErrorA(null);
    setErrorB(null);
    setStreamingA(true);
    setStreamingB(true);
    doneA.current = false;
    doneB.current = false;

    const checkComplete = () => {
      if (doneA.current && doneB.current) {
        onComplete();
      }
    };

    // Stream A
    (async () => {
      try {
        const stream = sendMessage(modelA.slug, message);
        const reader = stream.getReader();
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          setResponseA((prev) => prev + value);
        }
      } catch (err) {
        setErrorA(err instanceof Error ? err.message : "Stream failed");
      } finally {
        setStreamingA(false);
        doneA.current = true;
        checkComplete();
      }
    })();

    // Stream B
    (async () => {
      try {
        const stream = sendMessage(modelB.slug, message);
        const reader = stream.getReader();
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          setResponseB((prev) => prev + value);
        }
      } catch (err) {
        setErrorB(err instanceof Error ? err.message : "Stream failed");
      } finally {
        setStreamingB(false);
        doneB.current = true;
        checkComplete();
      }
    })();
  }, [message, modelA.slug, modelB.slug, onComplete]);

  // Auto-scroll as content streams in
  useEffect(() => {
    scrollToBottom(colARef);
  }, [responseA, scrollToBottom]);

  useEffect(() => {
    scrollToBottom(colBRef);
  }, [responseB, scrollToBottom]);

  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
      {/* Column A */}
      <div className="flex flex-col rounded-xl border border-gray-800 bg-gray-900">
        <div className="flex items-center gap-2 border-b border-gray-800 px-4 py-3">
          <div className="h-2.5 w-2.5 rounded-full bg-coral-500" />
          <h3 className="text-sm font-semibold text-white">{modelA.name}</h3>
          {streamingA && (
            <Loader2 className="ml-auto h-4 w-4 animate-spin text-coral-400" />
          )}
        </div>
        <div
          ref={colARef}
          className="flex-1 overflow-y-auto p-4 text-sm text-gray-300 whitespace-pre-wrap min-h-[200px] max-h-[500px]"
        >
          {responseA}
          {!responseA && streamingA && (
            <span className="inline-flex gap-1 text-gray-500">
              <span className="animate-pulse">.</span>
              <span className="animate-pulse delay-100">.</span>
              <span className="animate-pulse delay-200">.</span>
            </span>
          )}
          {errorA && (
            <p className="text-red-400">Error: {errorA}</p>
          )}
        </div>
      </div>

      {/* Column B */}
      <div className="flex flex-col rounded-xl border border-gray-800 bg-gray-900">
        <div className="flex items-center gap-2 border-b border-gray-800 px-4 py-3">
          <div className="h-2.5 w-2.5 rounded-full bg-purple-500" />
          <h3 className="text-sm font-semibold text-white">{modelB.name}</h3>
          {streamingB && (
            <Loader2 className="ml-auto h-4 w-4 animate-spin text-purple-400" />
          )}
        </div>
        <div
          ref={colBRef}
          className="flex-1 overflow-y-auto p-4 text-sm text-gray-300 whitespace-pre-wrap min-h-[200px] max-h-[500px]"
        >
          {responseB}
          {!responseB && streamingB && (
            <span className="inline-flex gap-1 text-gray-500">
              <span className="animate-pulse">.</span>
              <span className="animate-pulse delay-100">.</span>
              <span className="animate-pulse delay-200">.</span>
            </span>
          )}
          {errorB && (
            <p className="text-red-400">Error: {errorB}</p>
          )}
        </div>
      </div>
    </div>
  );
}

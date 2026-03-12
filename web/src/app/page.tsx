"use client";

import { useEffect, useState } from "react";
import { getModels, type Model } from "@/lib/api";
import ModelCard from "@/components/ModelCard";
import { ArrowRight, Sparkles, Shield, Zap } from "lucide-react";
import Link from "next/link";

export default function Home() {
  const [featured, setFeatured] = useState<Model[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getModels({ sort: "popular", limit: 6 })
      .then((res) => setFeatured(res.models))
      .catch(() => setFeatured([]))
      .finally(() => setLoading(false));
  }, []);

  return (
    <div>
      {/* Hero */}
      <section className="relative overflow-hidden border-b border-gray-800">
        <div className="absolute inset-0 bg-gradient-to-br from-coral-600/10 via-transparent to-purple-600/10" />
        <div className="relative mx-auto max-w-7xl px-4 py-24 text-center">
          <h1 className="mb-4 text-5xl font-bold tracking-tight md:text-6xl">
            AI models by your
            <br />
            <span className="bg-gradient-to-r from-coral-400 to-purple-400 bg-clip-text text-transparent">
              favorite creators
            </span>
          </h1>
          <p className="mx-auto mb-8 max-w-2xl text-lg text-gray-400">
            Chat with custom AI models trained by top creators. Pay per query,
            earn as a creator. Built on Solana.
          </p>
          <div className="flex justify-center gap-4">
            <Link
              href="/models"
              className="inline-flex items-center gap-2 rounded-xl bg-gradient-to-r from-coral-500 to-purple-500 px-6 py-3 font-semibold text-white transition hover:from-coral-400 hover:to-purple-400"
            >
              Browse Models
              <ArrowRight className="h-4 w-4" />
            </Link>
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="mx-auto max-w-7xl px-4 py-16">
        <div className="grid gap-8 md:grid-cols-3">
          {[
            {
              icon: Sparkles,
              title: "Creator-Trained",
              desc: "Models fine-tuned with unique knowledge and personality from top creators.",
            },
            {
              icon: Zap,
              title: "Pay Per Query",
              desc: "No subscriptions. Pay only for what you use with USDC on Solana.",
            },
            {
              icon: Shield,
              title: "Earn as a Creator",
              desc: "Build and monetize your own AI models. Keep the majority of revenue.",
            },
          ].map((f) => (
            <div
              key={f.title}
              className="rounded-xl border border-gray-800 bg-gray-900/50 p-6"
            >
              <f.icon className="mb-3 h-8 w-8 text-coral-400" />
              <h3 className="mb-2 text-lg font-semibold">{f.title}</h3>
              <p className="text-sm text-gray-400">{f.desc}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Featured Models */}
      <section className="mx-auto max-w-7xl px-4 pb-16">
        <div className="mb-8 flex items-center justify-between">
          <h2 className="text-2xl font-bold">Featured Models</h2>
          <Link
            href="/models"
            className="flex items-center gap-1 text-sm text-coral-400 hover:text-coral-300"
          >
            View all <ArrowRight className="h-3.5 w-3.5" />
          </Link>
        </div>
        {loading ? (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <div
                key={i}
                className="h-48 animate-pulse rounded-xl bg-gray-900"
              />
            ))}
          </div>
        ) : featured.length > 0 ? (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {featured.map((model) => (
              <ModelCard key={model.id} model={model} />
            ))}
          </div>
        ) : (
          <div className="rounded-xl border border-gray-800 bg-gray-900/50 py-16 text-center">
            <p className="text-gray-500">
              No models yet. Be the first creator to publish!
            </p>
          </div>
        )}
      </section>
    </div>
  );
}

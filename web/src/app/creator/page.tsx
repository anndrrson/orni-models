"use client";

import { useEffect, useState } from "react";
import {
  getCreatorStats,
  getCreatorModels,
  getCreatorEarnings,
  publishModel,
  toggleModelStatus,
  type CreatorStats,
  type Model,
  type EarningsData,
} from "@/lib/api";
import { useAuth } from "@/lib/wallet-provider";
import Link from "next/link";
import {
  Plus,
  BarChart3,
  MessageSquare,
  DollarSign,
  Bot,
  Eye,
  EyeOff,
  TrendingUp,
} from "lucide-react";
import { useRouter } from "next/navigation";

export default function CreatorDashboard() {
  const { authenticated, isCreator } = useAuth();
  const router = useRouter();
  const [stats, setStats] = useState<CreatorStats | null>(null);
  const [models, setModels] = useState<Model[]>([]);
  const [earnings, setEarnings] = useState<EarningsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [toggling, setToggling] = useState<string | null>(null);

  const fetchData = () => {
    if (!authenticated) return;
    Promise.all([getCreatorStats(), getCreatorModels(), getCreatorEarnings()])
      .then(([s, m, e]) => {
        setStats(s);
        setModels(m);
        setEarnings(e);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    fetchData();
  }, [authenticated]);

  const handleToggleStatus = async (model: Model) => {
    setToggling(model.id);
    try {
      const newStatus = model.status === "live" ? "paused" : "live";
      if (model.status === "draft") {
        await publishModel(model.id);
      } else {
        await toggleModelStatus(model.id, newStatus);
      }
      fetchData();
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to update status");
    } finally {
      setToggling(null);
    }
  };

  const formatMicro = (micro: number) =>
    `$${(micro / 1_000_000).toFixed(2)}`;

  if (!authenticated) {
    return (
      <div className="mx-auto max-w-7xl px-4 py-16 text-center">
        <h1 className="mb-4 text-2xl font-bold">Creator Dashboard</h1>
        <p className="text-gray-400">
          Connect your wallet to access your dashboard.
        </p>
      </div>
    );
  }

  if (!isCreator) {
    return (
      <div className="mx-auto max-w-7xl px-4 py-16 text-center">
        <h1 className="mb-4 text-2xl font-bold">Become a Creator</h1>
        <p className="mb-6 text-gray-400">
          Create your first AI model to get started!
        </p>
        <Link
          href="/creator/models/new"
          className="inline-flex items-center gap-2 rounded-xl bg-gradient-to-r from-coral-500 to-purple-500 px-6 py-2.5 text-sm font-semibold text-white hover:from-coral-400 hover:to-purple-400"
        >
          <Plus className="h-4 w-4" />
          Create Your First Model
        </Link>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-7xl px-4 py-8">
      <div className="mb-8 flex items-center justify-between">
        <h1 className="text-3xl font-bold">Creator Dashboard</h1>
        <Link
          href="/creator/models/new"
          className="inline-flex items-center gap-2 rounded-xl bg-gradient-to-r from-coral-500 to-purple-500 px-5 py-2.5 text-sm font-semibold text-white hover:from-coral-400 hover:to-purple-400"
        >
          <Plus className="h-4 w-4" />
          Create Model
        </Link>
      </div>

      {/* Stats */}
      {loading ? (
        <div className="mb-8 grid gap-4 md:grid-cols-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div
              key={i}
              className="h-28 animate-pulse rounded-xl bg-gray-900"
            />
          ))}
        </div>
      ) : (
        stats && (
          <div className="mb-8 grid gap-4 md:grid-cols-4">
            {[
              { label: "Total Models", value: stats.total_models, icon: Bot },
              {
                label: "Total Queries",
                value: stats.total_queries.toLocaleString(),
                icon: MessageSquare,
              },
              {
                label: "Total Revenue",
                value: formatMicro(stats.total_revenue),
                icon: BarChart3,
              },
              {
                label: "Your Earnings",
                value: formatMicro(stats.pending_earnings),
                icon: DollarSign,
              },
            ].map((s) => (
              <div
                key={s.label}
                className="rounded-xl border border-gray-800 bg-gray-900 p-5"
              >
                <s.icon className="mb-2 h-5 w-5 text-coral-400" />
                <p className="text-2xl font-bold">{s.value}</p>
                <p className="text-xs text-gray-500">{s.label}</p>
              </div>
            ))}
          </div>
        )
      )}

      {/* Earnings Chart */}
      {earnings && earnings.daily.length > 0 && (
        <div className="mb-8 rounded-xl border border-gray-800 bg-gray-900 p-6">
          <h2 className="mb-4 flex items-center gap-2 text-lg font-semibold">
            <TrendingUp className="h-5 w-5 text-coral-400" />
            Earnings (Last 30 Days)
          </h2>
          <div className="flex h-32 items-end gap-1">
            {earnings.daily.map((d) => {
              const maxAmount = Math.max(
                ...earnings.daily.map((x) => x.amount),
                1
              );
              const height = (d.amount / maxAmount) * 100;
              return (
                <div
                  key={d.date}
                  className="group relative flex-1"
                  title={`${d.date}: ${formatMicro(d.amount)}`}
                >
                  <div
                    className="w-full rounded-t bg-gradient-to-t from-coral-600 to-coral-400 transition-colors hover:from-coral-500 hover:to-coral-300"
                    style={{ height: `${Math.max(height, 2)}%` }}
                  />
                </div>
              );
            })}
          </div>
          <div className="mt-2 flex justify-between text-xs text-gray-500">
            <span>{earnings.daily[0]?.date}</span>
            <span>{earnings.daily[earnings.daily.length - 1]?.date}</span>
          </div>

          {/* Per-model breakdown */}
          {earnings.per_model.length > 0 && (
            <div className="mt-6">
              <h3 className="mb-3 text-sm font-medium text-gray-400">
                Per-Model Breakdown
              </h3>
              <div className="space-y-2">
                {earnings.per_model.map((m) => (
                  <div
                    key={m.model_id}
                    className="flex items-center justify-between rounded-lg bg-gray-800 px-4 py-2 text-sm"
                  >
                    <Link
                      href={`/models/${m.model_slug}`}
                      className="font-medium text-coral-400 hover:text-coral-300"
                    >
                      {m.model_name}
                    </Link>
                    <div className="flex items-center gap-4 text-gray-400">
                      <span>{m.query_count} queries</span>
                      <span className="font-medium text-white">
                        {formatMicro(m.creator_earnings)}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-3 flex justify-between border-t border-gray-700 pt-3 text-sm">
                <span className="text-gray-400">
                  Split: 85% creator / 15% platform
                </span>
                <span className="font-medium text-coral-400">
                  Total: {formatMicro(earnings.total_earnings)}
                </span>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Models List */}
      <h2 className="mb-4 text-xl font-semibold">Your Models</h2>
      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div
              key={i}
              className="h-20 animate-pulse rounded-xl bg-gray-900"
            />
          ))}
        </div>
      ) : models.length > 0 ? (
        <div className="space-y-3">
          {models.map((m) => (
            <div
              key={m.id}
              className="flex items-center justify-between rounded-xl border border-gray-800 bg-gray-900 p-4"
            >
              <div className="flex items-center gap-4">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-coral-500/10">
                  <Bot className="h-5 w-5 text-coral-400" />
                </div>
                <div>
                  <h3 className="font-semibold">{m.name}</h3>
                  <p className="text-xs text-gray-500">
                    {m.total_queries.toLocaleString()} queries |{" "}
                    {formatMicro(m.price_per_query)}/query
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <span
                  className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${
                    m.status === "live"
                      ? "bg-green-500/10 text-green-400"
                      : m.status === "draft"
                        ? "bg-yellow-500/10 text-yellow-400"
                        : m.status === "paused"
                          ? "bg-orange-500/10 text-orange-400"
                          : "bg-gray-500/10 text-gray-400"
                  }`}
                >
                  {m.status}
                </span>
                <button
                  onClick={() => handleToggleStatus(m)}
                  disabled={toggling === m.id}
                  className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition ${
                    m.status === "live"
                      ? "bg-orange-500/10 text-orange-400 hover:bg-orange-500/20"
                      : "bg-green-500/10 text-green-400 hover:bg-green-500/20"
                  } disabled:opacity-50`}
                  title={
                    m.status === "live" ? "Pause model" : "Publish model"
                  }
                >
                  {m.status === "live" ? (
                    <>
                      <EyeOff className="h-3.5 w-3.5" /> Unpublish
                    </>
                  ) : (
                    <>
                      <Eye className="h-3.5 w-3.5" /> Publish
                    </>
                  )}
                </button>
                <Link
                  href={`/models/${m.slug}`}
                  className="rounded-lg bg-gray-800 px-3 py-1.5 text-xs text-gray-300 hover:bg-gray-700"
                >
                  View
                </Link>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="rounded-xl border border-gray-800 bg-gray-900/50 py-12 text-center">
          <p className="mb-4 text-gray-500">
            You haven&apos;t created any models yet.
          </p>
          <Link
            href="/creator/models/new"
            className="inline-flex items-center gap-2 rounded-xl bg-coral-600 px-5 py-2.5 text-sm font-semibold text-white hover:bg-coral-500"
          >
            <Plus className="h-4 w-4" />
            Create Your First Model
          </Link>
        </div>
      )}
    </div>
  );
}

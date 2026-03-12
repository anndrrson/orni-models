import Link from "next/link";
import { MessageSquare, Star } from "lucide-react";
import type { Model } from "@/lib/api";

export default function ModelCard({ model }: { model: Model }) {
  return (
    <Link href={`/models/${model.slug}`}>
      <div className="group rounded-xl border border-gray-800 bg-gray-900 p-5 transition hover:border-coral-500/50 hover:shadow-lg hover:shadow-coral-500/5">
        <div className="mb-3 flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-gradient-to-br from-coral-500 to-purple-500 text-sm font-bold text-white">
              {model.creator_name?.[0]?.toUpperCase() || "?"}
            </div>
            <div>
              <h3 className="font-semibold text-white group-hover:text-coral-300 transition">
                {model.name}
              </h3>
              <p className="text-xs text-gray-500">{model.creator_name}</p>
            </div>
          </div>
          <div className="flex flex-col items-end gap-1">
            <span className="rounded-full bg-coral-500/10 px-2.5 py-0.5 text-xs font-medium text-coral-400">
              {model.category}
            </span>
            {model.free_queries_per_day && model.free_queries_per_day > 0 && (
              <span className="rounded-full bg-green-500/10 px-2.5 py-0.5 text-xs font-medium text-green-400">
                {model.free_queries_per_day} free/day
              </span>
            )}
          </div>
        </div>
        <p className="mb-4 line-clamp-2 text-sm text-gray-400">
          {model.description}
        </p>
        <div className="flex items-center justify-between text-xs text-gray-500">
          <div className="flex items-center gap-3">
            <span className="flex items-center gap-1">
              <MessageSquare className="h-3.5 w-3.5" />
              {model.total_queries.toLocaleString()}
            </span>
            {model.avg_rating > 0 && (
              <span className="flex items-center gap-1">
                <Star className="h-3.5 w-3.5 text-yellow-400 fill-yellow-400" />
                {model.avg_rating.toFixed(1)}
                {model.review_count > 0 && (
                  <span className="text-gray-600">({model.review_count})</span>
                )}
              </span>
            )}
          </div>
          <span className="font-medium text-coral-400">
            {model.free_queries_per_day && model.free_queries_per_day > 0
              ? `${model.free_queries_per_day} free/day`
              : `$${(model.price_per_query / 1_000_000).toFixed(2)}/query`}
          </span>
        </div>
      </div>
    </Link>
  );
}

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api";

function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("orni_token");
}

function setToken(token: string) {
  localStorage.setItem("orni_token", token);
}

export function clearToken() {
  localStorage.removeItem("orni_token");
}

async function apiFetch<T>(path: string, options: RequestInit = {}): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...((options.headers as Record<string, string>) || {}),
  };
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }
  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: `API error ${res.status}` }));
    throw new Error(body.error || `API error ${res.status}`);
  }
  const text = await res.text();
  if (!text) return undefined as T;
  return JSON.parse(text);
}

// ── Auth ──

export async function getNonce(wallet: string) {
  return apiFetch<{ nonce: string; message: string }>("/auth/nonce", {
    method: "POST",
    body: JSON.stringify({ wallet_address: wallet }),
  });
}

export async function verifySignature(wallet: string, signature: string, nonce: string) {
  const res = await apiFetch<{ token: string; user: { id: string; is_creator: boolean } }>(
    "/auth/verify",
    {
      method: "POST",
      body: JSON.stringify({ wallet_address: wallet, signature, nonce }),
    }
  );
  setToken(res.token);
  return { token: res.token, is_creator: res.user.is_creator };
}

// ── Models ──

export interface Model {
  id: string;
  slug: string;
  name: string;
  description?: string;
  avatar_url?: string;
  creator_name?: string;
  creator_wallet?: string;
  system_prompt?: string;
  price_per_query: number;
  category?: string;
  tags?: string[];
  total_queries: number;
  status: string;
  created_at: string;
}

export interface ModelsResponse {
  models: Model[];
  total: number;
  page: number;
  limit: number;
}

export async function getModels(params?: {
  search?: string;
  category?: string;
  sort?: string;
  page?: number;
  limit?: number;
}) {
  const sp = new URLSearchParams();
  if (params?.search) sp.set("search", params.search);
  if (params?.category) sp.set("category", params.category);
  if (params?.sort) sp.set("sort", params.sort);
  if (params?.page) sp.set("page", String(params.page));
  if (params?.limit) sp.set("limit", String(params.limit));
  const qs = sp.toString();
  return apiFetch<ModelsResponse>(`/models${qs ? `?${qs}` : ""}`);
}

export async function getModel(slug: string) {
  return apiFetch<Model>(`/models/${slug}`);
}

export async function createModel(data: {
  name: string;
  slug: string;
  description?: string;
  system_prompt: string;
  price_per_query?: number;
  category?: string;
}) {
  return apiFetch<Model>("/models/create", {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function updateModel(id: string, data: Partial<Model>) {
  return apiFetch<Model>(`/models/id/${id}`, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

// ── Chat (SSE streaming) ──

export function sendMessage(
  slug: string,
  message: string,
  sessionId?: string
): ReadableStream<string> {
  const token = getToken();
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (token) headers["Authorization"] = `Bearer ${token}`;

  return new ReadableStream<string>({
    async start(controller) {
      try {
        const res = await fetch(`${API_BASE}/chat/${slug}/message`, {
          method: "POST",
          headers,
          body: JSON.stringify({ message, session_id: sessionId }),
        });
        if (!res.ok) {
          const err = await res.text().catch(() => "Chat error");
          controller.error(new Error(err));
          return;
        }
        const reader = res.body?.getReader();
        if (!reader) {
          controller.close();
          return;
        }
        const decoder = new TextDecoder();
        let buffer = "";
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split("\n");
          buffer = lines.pop() || "";
          for (const line of lines) {
            if (line.startsWith("data: ")) {
              const data = line.slice(6);
              if (data === "[DONE]") {
                controller.close();
                return;
              }
              try {
                const parsed = JSON.parse(data);
                if (parsed.content) controller.enqueue(parsed.content);
                if (parsed.session_id && !sessionId) {
                  // Store session_id for follow-up messages
                  controller.enqueue(`__session:${parsed.session_id}`);
                }
              } catch {
                controller.enqueue(data);
              }
            }
          }
        }
        controller.close();
      } catch (e) {
        controller.error(e);
      }
    },
  });
}

// ── Balance & Payments ──

export async function getBalance() {
  return apiFetch<{ balance: number; pending_earnings: number }>("/balance");
}

export async function submitDeposit(txSignature: string, amount: number) {
  return apiFetch<{ id: string; amount: number }>("/deposits", {
    method: "POST",
    body: JSON.stringify({ tx_signature: txSignature, amount }),
  });
}

export async function requestWithdraw(amount: number, destinationWallet: string) {
  return apiFetch<{ status: string; message: string }>("/withdraw", {
    method: "POST",
    body: JSON.stringify({ amount, destination_wallet: destinationWallet }),
  });
}

// ── Creator ──

export interface CreatorStats {
  total_models: number;
  total_queries: number;
  total_revenue: number;
  pending_earnings: number;
}

export async function getCreatorStats() {
  return apiFetch<CreatorStats>("/creator/stats");
}

export async function getCreatorModels() {
  return apiFetch<Model[]>("/creator/models");
}

export async function addContent(
  modelId: string,
  data: { source_type: string; content_text?: string; source_url?: string }
) {
  return apiFetch(`/models/id/${modelId}/content`, {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function startFineTune(modelId: string) {
  return apiFetch(`/creator/models/${modelId}/fine-tune`, { method: "POST" });
}

// ── Namespace export for pages that use api.method() ──

export const api = {
  getNonce,
  verifySignature,
  getModels,
  getModel,
  createModel,
  updateModel,
  sendMessage,
  getBalance,
  submitDeposit,
  requestWithdraw,
  getCreatorStats,
  getCreatorModels,
  addContent,
  startFineTune,
  clearToken,
};

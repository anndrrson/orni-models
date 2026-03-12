'use client';

import { useState, useEffect, useCallback } from 'react';
import { Wallet, ArrowDownToLine, ArrowUpFromLine, RefreshCw, CreditCard, Key, Copy, Trash2, Plus } from 'lucide-react';
import { api, createCheckout, listApiKeys, createApiKey, revokeApiKey, getModels, type ApiKeyInfo, type Model } from '@/lib/api';

export default function AccountPage() {
  const [balance, setBalance] = useState<{ balance: number; pending_earnings: number } | null>(null);
  const [depositAmount, setDepositAmount] = useState('');
  const [depositTx, setDepositTx] = useState('');
  const [withdrawAmount, setWithdrawAmount] = useState('');
  const [loading, setLoading] = useState(false);
  const [checkoutLoading, setCheckoutLoading] = useState<string | null>(null);

  // API Keys state
  const [apiKeys, setApiKeys] = useState<ApiKeyInfo[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [newKeyModel, setNewKeyModel] = useState('');
  const [newKeyName, setNewKeyName] = useState('');
  const [newKeyResult, setNewKeyResult] = useState<string | null>(null);
  const [keyLoading, setKeyLoading] = useState(false);

  const fetchBalance = useCallback(async () => {
    try {
      const data = await api.getBalance();
      setBalance(data);
    } catch { /* not logged in */ }
  }, []);

  const fetchApiKeys = useCallback(async () => {
    try {
      const keys = await listApiKeys();
      setApiKeys(keys);
    } catch { /* not logged in */ }
  }, []);

  const fetchModels = useCallback(async () => {
    try {
      const data = await getModels({ limit: 100 });
      setModels(data.models);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    fetchBalance();
    fetchApiKeys();
    fetchModels();
  }, [fetchBalance, fetchApiKeys, fetchModels]);

  const formatUSDC = (micro: number) => `$${(micro / 1_000_000).toFixed(2)}`;

  const handleDeposit = async () => {
    if (!depositTx || !depositAmount) return;
    setLoading(true);
    try {
      await api.submitDeposit(depositTx, Math.round(parseFloat(depositAmount) * 1_000_000));
      setDepositTx(''); setDepositAmount('');
      await fetchBalance();
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : 'Deposit failed');
    } finally { setLoading(false); }
  };

  const handleWithdraw = async () => {
    if (!withdrawAmount) return;
    setLoading(true);
    try {
      await api.requestWithdraw(Math.round(parseFloat(withdrawAmount) * 1_000_000), '');
      setWithdrawAmount('');
      await fetchBalance();
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : 'Withdrawal failed');
    } finally { setLoading(false); }
  };

  const handleCheckout = async (pack: string) => {
    setCheckoutLoading(pack);
    try {
      const { checkout_url } = await createCheckout(pack);
      window.location.href = checkout_url;
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : 'Checkout failed');
    } finally {
      setCheckoutLoading(null);
    }
  };

  const handleCreateKey = async () => {
    if (!newKeyModel) return;
    setKeyLoading(true);
    setNewKeyResult(null);
    try {
      const result = await createApiKey(newKeyModel, newKeyName || undefined);
      setNewKeyResult(result.key);
      setNewKeyName('');
      await fetchApiKeys();
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : 'Failed to create key');
    } finally {
      setKeyLoading(false);
    }
  };

  const handleRevokeKey = async (id: string) => {
    if (!confirm('Revoke this API key? This cannot be undone.')) return;
    try {
      await revokeApiKey(id);
      await fetchApiKeys();
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : 'Failed to revoke key');
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  return (
    <div className="max-w-2xl mx-auto py-12 px-4">
      <h1 className="text-3xl font-bold mb-8">Account</h1>

      <div className="bg-gradient-to-br from-coral-500/20 to-purple-500/20 border border-coral-500/30 rounded-2xl p-8 mb-8">
        <div className="flex items-center gap-3 mb-4">
          <Wallet className="w-6 h-6 text-coral-400" />
          <span className="text-gray-400 text-sm font-medium">USDC Balance</span>
          <button onClick={fetchBalance} className="ml-auto text-gray-500 hover:text-white"><RefreshCw className="w-4 h-4" /></button>
        </div>
        <p className="text-5xl font-bold text-white mb-2">{balance ? formatUSDC(balance.balance) : '\u2014'}</p>
        {balance && balance.pending_earnings > 0 && (
          <p className="text-sm text-coral-300">+{formatUSDC(balance.pending_earnings)} creator earnings</p>
        )}
      </div>

      {/* API Keys */}
      <div className="bg-gray-900 rounded-xl p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Key className="w-5 h-5 text-purple-400" /> API Keys
        </h2>
        <p className="text-sm text-gray-400 mb-4">
          Create API keys to access models via the OpenAI-compatible endpoint.
        </p>

        {/* Create new key */}
        <div className="flex gap-2 mb-4">
          <select
            value={newKeyModel}
            onChange={(e) => setNewKeyModel(e.target.value)}
            className="flex-1 bg-gray-800 border border-gray-700 rounded-xl px-3 py-2 text-sm text-white focus:border-purple-500 focus:outline-none"
          >
            <option value="">Select a model...</option>
            {models.filter(m => m.status === 'live').map((m) => (
              <option key={m.id} value={m.id}>{m.name}</option>
            ))}
          </select>
          <input
            type="text"
            value={newKeyName}
            onChange={(e) => setNewKeyName(e.target.value)}
            placeholder="Key name (optional)"
            className="w-40 bg-gray-800 border border-gray-700 rounded-xl px-3 py-2 text-sm text-white focus:border-purple-500 focus:outline-none"
          />
          <button
            onClick={handleCreateKey}
            disabled={!newKeyModel || keyLoading}
            className="flex items-center gap-1.5 rounded-xl bg-purple-600 px-4 py-2 text-sm font-medium text-white hover:bg-purple-500 disabled:opacity-50"
          >
            <Plus className="w-4 h-4" /> Create
          </button>
        </div>

        {/* New key result */}
        {newKeyResult && (
          <div className="mb-4 rounded-xl border border-green-500/30 bg-green-500/10 p-4">
            <p className="text-xs text-green-300 mb-2">API key created. Copy it now — it won&apos;t be shown again:</p>
            <div className="flex items-center gap-2">
              <code className="flex-1 rounded-lg bg-gray-800 px-3 py-2 text-sm font-mono text-green-400 break-all">
                {newKeyResult}
              </code>
              <button
                onClick={() => copyToClipboard(newKeyResult)}
                className="rounded-lg bg-gray-700 p-2 hover:bg-gray-600"
                title="Copy"
              >
                <Copy className="w-4 h-4 text-gray-300" />
              </button>
            </div>
          </div>
        )}

        {/* Existing keys */}
        {apiKeys.length > 0 ? (
          <div className="space-y-2">
            {apiKeys.map((key) => (
              <div key={key.id} className="flex items-center justify-between rounded-lg bg-gray-800 px-4 py-3">
                <div>
                  <div className="flex items-center gap-2">
                    <code className="text-sm font-mono text-gray-300">{key.key_prefix}...</code>
                    {key.name && <span className="text-xs text-gray-500">{key.name}</span>}
                    {!key.is_active && <span className="text-xs text-red-400">revoked</span>}
                  </div>
                  <p className="text-xs text-gray-500 mt-0.5">
                    {key.model_name} &middot; Created {new Date(key.created_at).toLocaleDateString()}
                    {key.last_used_at && ` \u00b7 Last used ${new Date(key.last_used_at).toLocaleDateString()}`}
                  </p>
                </div>
                {key.is_active && (
                  <button
                    onClick={() => handleRevokeKey(key.id)}
                    className="rounded-lg p-2 text-gray-500 hover:bg-gray-700 hover:text-red-400"
                    title="Revoke key"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p className="text-sm text-gray-600">No API keys yet.</p>
        )}
      </div>

      <div className="bg-gray-900 rounded-xl p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <CreditCard className="w-5 h-5 text-blue-400" /> Buy Credits with Card
        </h2>
        <p className="text-sm text-gray-400 mb-4">Purchase credits instantly with a credit or debit card.</p>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {[
            { pack: '5', label: '$5', credits: '5M' },
            { pack: '10', label: '$10', credits: '10M' },
            { pack: '25', label: '$25', credits: '25M' },
            { pack: '50', label: '$50', credits: '50M' },
          ].map((p) => (
            <button
              key={p.pack}
              onClick={() => handleCheckout(p.pack)}
              disabled={!!checkoutLoading}
              className="flex flex-col items-center gap-1 rounded-xl border border-gray-700 bg-gray-800 p-4 transition hover:border-blue-500/50 hover:bg-gray-750 disabled:opacity-50"
            >
              <span className="text-xl font-bold text-white">{p.label}</span>
              <span className="text-xs text-gray-500">{p.credits} credits</span>
              {checkoutLoading === p.pack && <span className="text-xs text-blue-400">Redirecting...</span>}
            </button>
          ))}
        </div>
      </div>

      <div className="bg-gray-900 rounded-xl p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <ArrowDownToLine className="w-5 h-5 text-green-400" /> Deposit USDC
        </h2>
        <p className="text-sm text-gray-400 mb-4">Send USDC to the platform escrow wallet on Solana, then paste the transaction signature below.</p>
        <div className="space-y-3">
          <input type="number" step="0.01" placeholder="Amount (USDC)" value={depositAmount} onChange={(e) => setDepositAmount(e.target.value)} className="w-full bg-gray-800 border border-gray-700 rounded-xl px-4 py-3 text-white focus:border-coral-500 focus:outline-none" />
          <input type="text" placeholder="Transaction signature" value={depositTx} onChange={(e) => setDepositTx(e.target.value)} className="w-full bg-gray-800 border border-gray-700 rounded-xl px-4 py-3 text-white focus:border-coral-500 focus:outline-none font-mono text-sm" />
          <button onClick={handleDeposit} disabled={loading || !depositTx || !depositAmount} className="w-full py-3 bg-green-600 hover:bg-green-700 disabled:opacity-50 text-white rounded-xl font-medium transition-colors">
            {loading ? 'Verifying...' : 'Verify Deposit'}
          </button>
        </div>
      </div>

      <div className="bg-gray-900 rounded-xl p-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <ArrowUpFromLine className="w-5 h-5 text-orange-400" /> Withdraw USDC
        </h2>
        <div className="space-y-3">
          <input type="number" step="0.01" placeholder="Amount (USDC)" value={withdrawAmount} onChange={(e) => setWithdrawAmount(e.target.value)} className="w-full bg-gray-800 border border-gray-700 rounded-xl px-4 py-3 text-white focus:border-coral-500 focus:outline-none" />
          <button onClick={handleWithdraw} disabled={loading || !withdrawAmount} className="w-full py-3 bg-orange-600 hover:bg-orange-700 disabled:opacity-50 text-white rounded-xl font-medium transition-colors">
            {loading ? 'Processing...' : 'Withdraw'}
          </button>
        </div>
      </div>
    </div>
  );
}

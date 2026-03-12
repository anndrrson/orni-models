"use client";

import Link from "next/link";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { useAuth } from "@/lib/wallet-provider";
import { useEffect, useState } from "react";
import { getBalance } from "@/lib/api";
import { Bot, Menu, X } from "lucide-react";

export default function Navbar() {
  const { connected } = useWallet();
  const { authenticated, isCreator } = useAuth();
  const [balance, setBalance] = useState<number | null>(null);
  const [mobileOpen, setMobileOpen] = useState(false);

  useEffect(() => {
    if (authenticated) {
      getBalance()
        .then((b) => setBalance(b.balance))
        .catch(() => setBalance(null));
    } else {
      setBalance(null);
    }
  }, [authenticated]);

  return (
    <nav className="sticky top-0 z-50 border-b border-gray-800 bg-gray-950/80 backdrop-blur-md">
      <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4">
        <div className="flex items-center gap-8">
          <Link href="/" className="flex items-center gap-2 text-lg font-bold">
            <Bot className="h-6 w-6 text-coral-400" />
            <span className="bg-gradient-to-r from-coral-400 to-purple-400 bg-clip-text text-transparent">
              kinakuta
            </span>
          </Link>
          <div className="hidden items-center gap-6 md:flex">
            <Link
              href="/models"
              className="text-sm text-gray-400 transition hover:text-white"
            >
              Browse
            </Link>
            {isCreator && (
              <Link
                href="/creator"
                className="text-sm text-gray-400 transition hover:text-white"
              >
                Creator
              </Link>
            )}
          </div>
        </div>
        <div className="hidden items-center gap-4 md:flex">
          {authenticated && balance !== null && (
            <Link
              href="/account"
              className="rounded-lg bg-gray-800 px-3 py-1.5 text-sm font-medium text-gray-300 transition hover:bg-gray-700"
            >
              ${balance.toFixed(2)}
            </Link>
          )}
          <WalletMultiButton />
        </div>
        <button
          className="md:hidden text-gray-400"
          onClick={() => setMobileOpen(!mobileOpen)}
        >
          {mobileOpen ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
        </button>
      </div>
      {mobileOpen && (
        <div className="border-t border-gray-800 bg-gray-950 px-4 py-4 md:hidden">
          <div className="flex flex-col gap-4">
            <Link href="/models" className="text-sm text-gray-400" onClick={() => setMobileOpen(false)}>
              Browse
            </Link>
            {isCreator && (
              <Link href="/creator" className="text-sm text-gray-400" onClick={() => setMobileOpen(false)}>
                Creator
              </Link>
            )}
            {authenticated && balance !== null && (
              <Link href="/account" className="text-sm text-gray-400" onClick={() => setMobileOpen(false)}>
                Balance: ${balance.toFixed(2)}
              </Link>
            )}
            <WalletMultiButton />
          </div>
        </div>
      )}
    </nav>
  );
}

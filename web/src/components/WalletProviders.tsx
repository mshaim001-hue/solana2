"use client";

import { useMemo, ReactNode } from "react";
import {
  ConnectionProvider,
  WalletProvider,
} from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import { DEVNET_CONFIG } from "@/lib/config";
import "@solana/wallet-adapter-react-ui/styles.css";

export function WalletProviders({ children }: { children: ReactNode }) {
  const endpoint = DEVNET_CONFIG.rpcUrl;
  // Empty list → Standard Wallet API / browser wallets (Phantom etc.)
  const wallets = useMemo(() => [], []);
  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}

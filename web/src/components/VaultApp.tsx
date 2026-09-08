"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import {
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  getAccount,
  getMint,
} from "@solana/spl-token";
import { DEVNET_CONFIG } from "@/lib/config";
import {
  buildDepositIx,
  buildFundRewardsIx,
  buildWithdrawIx,
  ensureAtas,
  withComputeBudget,
} from "@/lib/instructions";
import {
  decodeVault,
  formatUnits,
  parseUnits,
  shareMintPda,
  vaultPda,
  type VaultAccount,
} from "@/lib/vault";

type TxStatus =
  | { kind: "idle" }
  | { kind: "pending"; label: string }
  | { kind: "success"; label: string; signature: string }
  | { kind: "error"; label: string; message: string };

function explorerTx(sig: string) {
  return `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
}

export function VaultApp() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction, connected } = useWallet();
  const [vault, setVault] = useState<VaultAccount | null>(null);
  const [underlyingBal, setUnderlyingBal] = useState<bigint>(0n);
  const [shareBal, setShareBal] = useState<bigint>(0n);
  const [decimals, setDecimals] = useState(6);
  const [depositAmt, setDepositAmt] = useState("10");
  const [withdrawShares, setWithdrawShares] = useState("5");
  const [fundAmt, setFundAmt] = useState("50");
  const [status, setStatus] = useState<TxStatus>({ kind: "idle" });
  const [mintInput, setMintInput] = useState(DEVNET_CONFIG.underlyingMint);

  const underlyingMint = useMemo(() => {
    try {
      return mintInput ? new PublicKey(mintInput) : null;
    } catch {
      return null;
    }
  }, [mintInput]);

  const refresh = useCallback(async () => {
    if (!underlyingMint) return;
    const vaultPk = vaultPda(underlyingMint);
    const shareMint = shareMintPda(vaultPk);
    const vaultInfo = await connection.getAccountInfo(vaultPk);
    if (!vaultInfo) {
      setVault(null);
      return;
    }
    const decoded = decodeVault(Buffer.from(vaultInfo.data));
    setVault(decoded);
    try {
      const mint = await getMint(connection, underlyingMint);
      setDecimals(mint.decimals);
    } catch {
      /* ignore */
    }
    if (publicKey) {
      try {
        const uAta = getAssociatedTokenAddressSync(underlyingMint, publicKey);
        const sAta = getAssociatedTokenAddressSync(shareMint, publicKey);
        const uAcc = await getAccount(connection, uAta);
        setUnderlyingBal(uAcc.amount);
      } catch {
        setUnderlyingBal(0n);
      }
      try {
        const sAta = getAssociatedTokenAddressSync(shareMint, publicKey);
        const sAcc = await getAccount(connection, sAta);
        setShareBal(sAcc.amount);
      } catch {
        setShareBal(0n);
      }
    }
  }, [connection, underlyingMint, publicKey]);

  useEffect(() => {
    refresh().catch(() => undefined);
    const id = setInterval(() => {
      refresh().catch(() => undefined);
    }, 12_000);
    return () => clearInterval(id);
  }, [refresh]);

  const send = async (label: string, build: () => Promise<Transaction>) => {
    if (!publicKey) {
      setStatus({ kind: "error", label, message: "Connect a wallet first" });
      return;
    }
    setStatus({ kind: "pending", label });
    try {
      const tx = await build();
      withComputeBudget(tx);
      const { blockhash, lastValidBlockHeight } =
        await connection.getLatestBlockhash("confirmed");
      tx.feePayer = publicKey;
      tx.recentBlockhash = blockhash;
      const signature = await sendTransaction(tx, connection);
      await connection.confirmTransaction(
        { signature, blockhash, lastValidBlockHeight },
        "confirmed"
      );
      setStatus({ kind: "success", label, signature });
      await refresh();
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      setStatus({ kind: "error", label, message });
    }
  };

  const onDeposit = () =>
    send("Deposit", async () => {
      if (!publicKey || !underlyingMint || !vault) throw new Error("Vault not ready");
      const amount = parseUnits(depositAmt, decimals);
      const shareMint = shareMintPda(vaultPda(underlyingMint));
      const atas = await ensureAtas(connection, publicKey, [
        { mint: underlyingMint, owner: publicKey },
        { mint: shareMint, owner: publicKey },
      ]);
      const tx = new Transaction().add(
        ...atas,
        buildDepositIx({ user: publicKey, underlyingMint, amount })
      );
      return tx;
    });

  const onWithdraw = () =>
    send("Withdraw", async () => {
      if (!publicKey || !underlyingMint || !vault) throw new Error("Vault not ready");
      const shares = parseUnits(withdrawShares, 6);
      const atas = await ensureAtas(connection, publicKey, [
        { mint: underlyingMint, owner: publicKey },
      ]);
      const tx = new Transaction().add(
        ...atas,
        buildWithdrawIx({
          user: publicKey,
          underlyingMint,
          shares,
        })
      );
      return tx;
    });

  const onFund = () =>
    send("Fund rewards", async () => {
      if (!publicKey || !underlyingMint || !vault) throw new Error("Vault not ready");
      if (!publicKey.equals(vault.authority)) {
        throw new Error("Only vault authority can fund rewards");
      }
      const amount = parseUnits(fundAmt, decimals);
      const tx = new Transaction().add(
        buildFundRewardsIx({
          authority: publicKey,
          underlyingMint,
          amount,
        })
      );
      return tx;
    });

  const exchangeRate =
    vault && vault.totalShares > 0n
      ? Number(vault.totalAssets) / Number(vault.totalShares)
      : 1;

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-10 px-5 py-10">
      <header className="flex flex-wrap items-end justify-between gap-6">
        <div>
          <p className="font-display text-4xl tracking-tight text-ink md:text-5xl">
            YieldVault
          </p>
          <p className="mt-2 max-w-xl text-base text-ink/70">
            Devnet staking vault: deposit SPL tokens, earn time-based APY shares,
            withdraw with a protocol fee. PDA-secured liquidity, no mainnet funds.
          </p>
        </div>
        <WalletMultiButton />
      </header>

      <section className="grid gap-4 rounded-2xl border border-line/20 bg-white/60 p-5 backdrop-blur">
        <label className="text-sm font-medium text-ink/80">
          Underlying mint (from setup script)
          <input
            className="mt-1 w-full rounded-lg border border-line/30 bg-white px-3 py-2 font-mono text-sm"
            value={mintInput}
            onChange={(e) => setMintInput(e.target.value.trim())}
            placeholder="Paste SPL mint address"
          />
        </label>
        <div className="flex flex-wrap gap-3 text-sm">
          <button
            type="button"
            onClick={() => refresh()}
            className="rounded-lg bg-ink px-4 py-2 text-white"
          >
            Refresh state
          </button>
          <a
            className="rounded-lg border border-line/40 px-4 py-2"
            href={`https://explorer.solana.com/address/${DEVNET_CONFIG.programId}?cluster=devnet`}
            target="_blank"
            rel="noreferrer"
          >
            Program on Explorer
          </a>
        </div>
      </section>

      <section className="grid gap-4 md:grid-cols-3">
        <Stat
          label="Total assets"
          value={vault ? formatUnits(vault.totalAssets, decimals) : "—"}
        />
        <Stat
          label="Total shares"
          value={vault ? formatUnits(vault.totalShares, 6) : "—"}
        />
        <Stat
          label="APY / fee"
          value={
            vault
              ? `${(vault.apyBps / 100).toFixed(2)}% / ${(vault.withdrawalFeeBps / 100).toFixed(2)}%`
              : "—"
          }
        />
        <Stat label="Exchange rate" value={vault ? exchangeRate.toFixed(6) : "—"} />
        <Stat
          label="Your underlying"
          value={connected ? formatUnits(underlyingBal, decimals) : "—"}
        />
        <Stat
          label="Your shares"
          value={connected ? formatUnits(shareBal, 6) : "—"}
        />
      </section>

      {!vault && (
        <p className="rounded-xl border border-warn/40 bg-warn/10 px-4 py-3 text-sm">
          Vault not found for this mint. Run{" "}
          <code className="font-mono">node scripts/setup-devnet.mjs</code> after
          deploying the program, then paste the mint address above.
        </p>
      )}

      <section className="grid gap-6 md:grid-cols-3">
        <ActionCard title="Deposit" hint="Mint vault shares 1:1 at start, then by exchange rate.">
          <input
            className="w-full rounded-lg border border-line/30 px-3 py-2"
            value={depositAmt}
            onChange={(e) => setDepositAmt(e.target.value)}
          />
          <button
            type="button"
            disabled={!connected || !vault}
            onClick={onDeposit}
            className="mt-3 w-full rounded-lg bg-accent px-4 py-2 font-medium text-white disabled:opacity-40"
          >
            Deposit
          </button>
        </ActionCard>

        <ActionCard title="Withdraw" hint="Burns shares; applies withdrawal fee to authority ATA.">
          <input
            className="w-full rounded-lg border border-line/30 px-3 py-2"
            value={withdrawShares}
            onChange={(e) => setWithdrawShares(e.target.value)}
          />
          <button
            type="button"
            disabled={!connected || !vault}
            onClick={onWithdraw}
            className="mt-3 w-full rounded-lg bg-ink px-4 py-2 font-medium text-white disabled:opacity-40"
          >
            Withdraw
          </button>
        </ActionCard>

        <ActionCard title="Fund rewards" hint="Authority tops up liquidity so accrued APY is redeemable.">
          <input
            className="w-full rounded-lg border border-line/30 px-3 py-2"
            value={fundAmt}
            onChange={(e) => setFundAmt(e.target.value)}
          />
          <button
            type="button"
            disabled={!connected || !vault}
            onClick={onFund}
            className="mt-3 w-full rounded-lg border border-accent px-4 py-2 font-medium text-accent disabled:opacity-40"
          >
            Fund
          </button>
        </ActionCard>
      </section>

      <StatusBanner status={status} />

      <footer className="border-t border-line/20 pt-6 text-sm text-ink/60">
        <p>
          Program ID:{" "}
          <span className="font-mono text-ink">{DEVNET_CONFIG.programId}</span>
        </p>
        <p className="mt-1">
          Educational Devnet MVP — synthetic APY, not production yield.
        </p>
      </footer>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-line/15 bg-white/70 px-4 py-4">
      <p className="text-xs uppercase tracking-wide text-ink/50">{label}</p>
      <p className="mt-1 font-display text-2xl text-ink">{value}</p>
    </div>
  );
}

function ActionCard({
  title,
  hint,
  children,
}: {
  title: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-2xl border border-line/20 bg-white/75 p-5">
      <h2 className="font-display text-xl">{title}</h2>
      <p className="mt-1 mb-3 text-sm text-ink/60">{hint}</p>
      {children}
    </div>
  );
}

function StatusBanner({ status }: { status: TxStatus }) {
  if (status.kind === "idle") return null;
  if (status.kind === "pending") {
    return (
      <div className="rounded-xl border border-accent/30 bg-accent/10 px-4 py-3 text-sm">
        Submitting {status.label}… confirm in your wallet.
      </div>
    );
  }
  if (status.kind === "success") {
    return (
      <div className="rounded-xl border border-accent/40 bg-accent/10 px-4 py-3 text-sm">
        {status.label} confirmed.{" "}
        <a
          className="underline"
          href={explorerTx(status.signature)}
          target="_blank"
          rel="noreferrer"
        >
          View on Explorer
        </a>
      </div>
    );
  }
  return (
    <div className="rounded-xl border border-warn/40 bg-warn/10 px-4 py-3 text-sm">
      {status.label} failed: {status.message}
    </div>
  );
}

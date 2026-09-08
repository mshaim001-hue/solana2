"use client";

import dynamic from "next/dynamic";

const VaultApp = dynamic(
  () => import("@/components/VaultApp").then((m) => m.VaultApp),
  {
    ssr: false,
    loading: () => (
      <div className="mx-auto max-w-5xl px-5 py-16 text-ink/60">
        Loading YieldVault…
      </div>
    ),
  }
);

export default function Home() {
  return <VaultApp />;
}

import type { Metadata } from "next";
import { Fraunces, Source_Sans_3 } from "next/font/google";
import { WalletProviders } from "@/components/WalletProviders";
import "./globals.css";

const display = Fraunces({
  subsets: ["latin"],
  variable: "--font-display",
});

const sans = Source_Sans_3({
  subsets: ["latin"],
  variable: "--font-sans",
});

export const metadata: Metadata = {
  title: "YieldVault — Solana Devnet DeFi MVP",
  description:
    "Staking / yield vault on Solana Devnet with PDA accounting, SPL shares, and wallet UX.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className={`${display.variable} ${sans.variable} font-sans antialiased`}>
        <WalletProviders>{children}</WalletProviders>
      </body>
    </html>
  );
}

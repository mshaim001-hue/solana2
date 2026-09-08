import { PublicKey } from "@solana/web3.js";
import {
  PROGRAM_ID,
  SHARE_MINT_SEED,
  VAULT_SEED,
  VAULT_TOKEN_SEED,
  FEE_TOKEN_SEED,
} from "./config";

export const programId = new PublicKey(PROGRAM_ID);

export function vaultPda(underlyingMint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, underlyingMint.toBuffer()],
    programId
  )[0];
}

export function shareMintPda(vault: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [SHARE_MINT_SEED, vault.toBuffer()],
    programId
  )[0];
}

export function vaultTokenPda(vault: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [VAULT_TOKEN_SEED, vault.toBuffer()],
    programId
  )[0];
}

export function feeTokenPda(vault: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [FEE_TOKEN_SEED, vault.toBuffer()],
    programId
  )[0];
}

export type VaultAccount = {
  authority: PublicKey;
  underlyingMint: PublicKey;
  shareMint: PublicKey;
  vaultToken: PublicKey;
  feeToken: PublicKey;
  totalAssets: bigint;
  totalShares: bigint;
  apyBps: number;
  withdrawalFeeBps: number;
  minDeposit: bigint;
  maxTvl: bigint;
  lastUpdateTs: bigint;
  paused: boolean;
  bump: number;
  vaultTokenBump: number;
  feeTokenBump: number;
  shareMintBump: number;
};

/** Manual layout decode (avoids Anchor client IDL version friction). */
export function decodeVault(data: Buffer): VaultAccount {
  let o = 8;
  const readPk = () => {
    const pk = new PublicKey(data.subarray(o, o + 32));
    o += 32;
    return pk;
  };
  const readU64 = () => {
    const v = data.readBigUInt64LE(o);
    o += 8;
    return v;
  };
  const readU16 = () => {
    const v = data.readUInt16LE(o);
    o += 2;
    return v;
  };
  const readI64 = () => {
    const v = data.readBigInt64LE(o);
    o += 8;
    return v;
  };

  return {
    authority: readPk(),
    underlyingMint: readPk(),
    shareMint: readPk(),
    vaultToken: readPk(),
    feeToken: readPk(),
    totalAssets: readU64(),
    totalShares: readU64(),
    apyBps: readU16(),
    withdrawalFeeBps: readU16(),
    minDeposit: readU64(),
    maxTvl: readU64(),
    lastUpdateTs: readI64(),
    paused: data[o++] === 1,
    bump: data[o++],
    vaultTokenBump: data[o++],
    feeTokenBump: data[o++],
    shareMintBump: data[o++],
  };
}

export function formatUnits(amount: bigint, decimals: number): string {
  const base = 10n ** BigInt(decimals);
  const whole = amount / base;
  const frac = amount % base;
  const fracStr = frac.toString().padStart(decimals, "0").replace(/0+$/, "");
  return fracStr ? `${whole}.${fracStr}` : whole.toString();
}

export function parseUnits(value: string, decimals: number): bigint {
  const trimmed = value.trim();
  if (!trimmed || Number(trimmed) < 0) throw new Error("Invalid amount");
  const [w, f = ""] = trimmed.split(".");
  const frac = (f + "0".repeat(decimals)).slice(0, decimals);
  return BigInt(w || "0") * 10n ** BigInt(decimals) + BigInt(frac || "0");
}

import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  Transaction,
  Connection,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotentInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import idl from "./yield_vault.json";
import { programId, shareMintPda, vaultPda, vaultTokenPda, feeTokenPda } from "./vault";

type IdlIx = {
  name: string;
  discriminator: number[];
};

function disc(name: string): Buffer {
  const ix = (idl as { instructions: IdlIx[] }).instructions.find(
    (i) => i.name === name
  );
  if (!ix) throw new Error(`Unknown instruction ${name}`);
  return Buffer.from(ix.discriminator);
}

function u64le(n: bigint): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(n);
  return b;
}

function u16le(n: number): Buffer {
  const b = Buffer.alloc(2);
  b.writeUInt16LE(n);
  return b;
}

export function buildInitializeIx(args: {
  authority: PublicKey;
  underlyingMint: PublicKey;
  apyBps: number;
  withdrawalFeeBps: number;
  minDeposit: bigint;
  maxTvl: bigint;
}): TransactionInstruction {
  const vault = vaultPda(args.underlyingMint);
  const shareMint = shareMintPda(vault);
  const vaultToken = vaultTokenPda(vault);
  const feeToken = feeTokenPda(vault);
  const data = Buffer.concat([
    disc("initialize"),
    u16le(args.apyBps),
    u16le(args.withdrawalFeeBps),
    u64le(args.minDeposit),
    u64le(args.maxTvl),
  ]);
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: args.authority, isSigner: true, isWritable: true },
      { pubkey: args.underlyingMint, isSigner: false, isWritable: false },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: shareMint, isSigner: false, isWritable: true },
      { pubkey: vaultToken, isSigner: false, isWritable: true },
      { pubkey: feeToken, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function buildDepositIx(args: {
  user: PublicKey;
  underlyingMint: PublicKey;
  amount: bigint;
}): TransactionInstruction {
  const vault = vaultPda(args.underlyingMint);
  const shareMint = shareMintPda(vault);
  const vaultToken = vaultTokenPda(vault);
  const userUnderlying = getAssociatedTokenAddressSync(
    args.underlyingMint,
    args.user
  );
  const userShares = getAssociatedTokenAddressSync(shareMint, args.user);
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: args.user, isSigner: true, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: args.underlyingMint, isSigner: false, isWritable: false },
      { pubkey: shareMint, isSigner: false, isWritable: true },
      { pubkey: vaultToken, isSigner: false, isWritable: true },
      { pubkey: userUnderlying, isSigner: false, isWritable: true },
      { pubkey: userShares, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([disc("deposit"), u64le(args.amount)]),
  });
}

export function buildWithdrawIx(args: {
  user: PublicKey;
  underlyingMint: PublicKey;
  shares: bigint;
}): TransactionInstruction {
  const vault = vaultPda(args.underlyingMint);
  const shareMint = shareMintPda(vault);
  const vaultToken = vaultTokenPda(vault);
  const feeToken = feeTokenPda(vault);
  const userUnderlying = getAssociatedTokenAddressSync(
    args.underlyingMint,
    args.user
  );
  const userShares = getAssociatedTokenAddressSync(shareMint, args.user);
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: args.user, isSigner: true, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: args.underlyingMint, isSigner: false, isWritable: false },
      { pubkey: shareMint, isSigner: false, isWritable: true },
      { pubkey: vaultToken, isSigner: false, isWritable: true },
      { pubkey: feeToken, isSigner: false, isWritable: true },
      { pubkey: userUnderlying, isSigner: false, isWritable: true },
      { pubkey: userShares, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([disc("withdraw"), u64le(args.shares)]),
  });
}

export function buildFundRewardsIx(args: {
  authority: PublicKey;
  underlyingMint: PublicKey;
  amount: bigint;
}): TransactionInstruction {
  const vault = vaultPda(args.underlyingMint);
  const vaultToken = vaultTokenPda(vault);
  const authorityUnderlying = getAssociatedTokenAddressSync(
    args.underlyingMint,
    args.authority
  );
  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: args.authority, isSigner: true, isWritable: false },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: args.underlyingMint, isSigner: false, isWritable: false },
      { pubkey: vaultToken, isSigner: false, isWritable: true },
      { pubkey: authorityUnderlying, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([disc("fund_rewards"), u64le(args.amount)]),
  });
}

export async function ensureAtas(
  connection: Connection,
  payer: PublicKey,
  mints: { mint: PublicKey; owner: PublicKey }[]
): Promise<TransactionInstruction[]> {
  const ixs: TransactionInstruction[] = [];
  for (const { mint, owner } of mints) {
    const ata = getAssociatedTokenAddressSync(mint, owner);
    const info = await connection.getAccountInfo(ata);
    if (!info) {
      ixs.push(
        createAssociatedTokenAccountIdempotentInstruction(
          payer,
          ata,
          owner,
          mint,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        )
      );
    }
  }
  return ixs;
}

export function withComputeBudget(tx: Transaction): Transaction {
  tx.instructions.unshift(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 })
  );
  return tx;
}

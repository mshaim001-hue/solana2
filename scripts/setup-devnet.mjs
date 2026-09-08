#!/usr/bin/env node
/**
 * Devnet bootstrap:
 * 1) Create SPL mint + ATA
 * 2) Initialize YieldVault
 * 3) Optional demo deposit / fund / withdraw
 * 4) Write web/src/lib/config.ts + demo-artifacts.json
 *
 * Usage:
 *   ANCHOR_WALLET=.keys/deployer.json node scripts/setup-devnet.mjs
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createInitializeMint2Instruction,
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  getMint,
} from "@solana/spl-token";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const idl = JSON.parse(
  fs.readFileSync(path.join(root, "target/idl/yield_vault.json"), "utf8")
);
const PROGRAM_ID = new PublicKey(idl.address);

function loadKeypair(p) {
  const raw = JSON.parse(fs.readFileSync(p, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

function disc(name) {
  const ix = idl.instructions.find((i) => i.name === name);
  if (!ix) throw new Error(`missing ix ${name}`);
  return Buffer.from(ix.discriminator);
}

function u64(n) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n));
  return b;
}
function u16(n) {
  const b = Buffer.alloc(2);
  b.writeUInt16LE(n);
  return b;
}

function pda(seeds) {
  return PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
}

async function main() {
  const walletPath =
    process.env.ANCHOR_WALLET || path.join(root, ".keys/deployer.json");
  const payer = loadKeypair(walletPath);
  const rpc = process.env.RPC_URL || "https://api.devnet.solana.com";
  const connection = new Connection(rpc, "confirmed");

  const bal = await connection.getBalance(payer.publicKey);
  console.log("Authority:", payer.publicKey.toBase58());
  console.log("Balance:", bal / 1e9, "SOL");
  if (bal < 0.5e9) {
    throw new Error(
      "Need >= 0.5 SOL on Devnet. Run: solana airdrop 2 --url devnet"
    );
  }

  const decimals = 6;
  const mintKp = Keypair.generate();
  const lamports = await connection.getMinimumBalanceForRentExemption(MINT_SIZE);

  const createMintTx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }),
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: mintKp.publicKey,
      space: MINT_SIZE,
      lamports,
      programId: TOKEN_PROGRAM_ID,
    }),
    createInitializeMint2Instruction(
      mintKp.publicKey,
      decimals,
      payer.publicKey,
      null,
      TOKEN_PROGRAM_ID
    )
  );
  const mintSig = await sendAndConfirmTransaction(connection, createMintTx, [
    payer,
    mintKp,
  ]);
  console.log("Mint:", mintKp.publicKey.toBase58(), mintSig);

  const ata = getAssociatedTokenAddressSync(mintKp.publicKey, payer.publicKey);
  const mintToTx = new Transaction().add(
    createAssociatedTokenAccountIdempotentInstruction(
      payer.publicKey,
      ata,
      payer.publicKey,
      mintKp.publicKey
    ),
    createMintToInstruction(
      mintKp.publicKey,
      ata,
      payer.publicKey,
      1_000_000_000_000n // 1_000_000 tokens
    )
  );
  await sendAndConfirmTransaction(connection, mintToTx, [payer]);

  const vault = pda([Buffer.from("vault"), mintKp.publicKey.toBuffer()]);
  const shareMint = pda([Buffer.from("share_mint"), vault.toBuffer()]);
  const vaultToken = pda([Buffer.from("vault_token"), vault.toBuffer()]);
  const feeToken = pda([Buffer.from("fee_token"), vault.toBuffer()]);

  const apyBps = 1000; // 10%
  const feeBps = 50; // 0.5%
  const minDeposit = 1_000_000n; // 1 token
  const maxTvl = 100_000_000_000_000n;

  const initData = Buffer.concat([
    disc("initialize"),
    u16(apyBps),
    u16(feeBps),
    u64(minDeposit),
    u64(maxTvl),
  ]);
  const initIx = {
    programId: PROGRAM_ID,
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: mintKp.publicKey, isSigner: false, isWritable: false },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: shareMint, isSigner: false, isWritable: true },
      { pubkey: vaultToken, isSigner: false, isWritable: true },
      { pubkey: feeToken, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: initData,
  };
  const initSig = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
      initIx
    ),
    [payer]
  );
  console.log("Initialize:", initSig);

  // Fund rewards + deposit demo
  const fundAmt = 100_000_000n; // 100 tokens
  const fundIx = {
    programId: PROGRAM_ID,
    keys: [
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: mintKp.publicKey, isSigner: false, isWritable: false },
      { pubkey: vaultToken, isSigner: false, isWritable: true },
      { pubkey: ata, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([disc("fund_rewards"), u64(fundAmt)]),
  };

  const userShares = getAssociatedTokenAddressSync(shareMint, payer.publicKey);
  const depositAmt = 50_000_000n; // 50 tokens
  const depositTx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
    createAssociatedTokenAccountIdempotentInstruction(
      payer.publicKey,
      userShares,
      payer.publicKey,
      shareMint
    ),
    fundIx,
    {
      programId: PROGRAM_ID,
      keys: [
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: mintKp.publicKey, isSigner: false, isWritable: false },
        { pubkey: shareMint, isSigner: false, isWritable: true },
        { pubkey: vaultToken, isSigner: false, isWritable: true },
        { pubkey: ata, isSigner: false, isWritable: true },
        { pubkey: userShares, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data: Buffer.concat([disc("deposit"), u64(depositAmt)]),
    }
  );
  const depositSig = await sendAndConfirmTransaction(connection, depositTx, [
    payer,
  ]);
  console.log("Fund+Deposit:", depositSig);

  // Small withdraw
  const withdrawShares = 5_000_000n; // 5 share units (6 decimals)
  const withdrawTx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
    {
      programId: PROGRAM_ID,
      keys: [
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: mintKp.publicKey, isSigner: false, isWritable: false },
        { pubkey: shareMint, isSigner: false, isWritable: true },
        { pubkey: vaultToken, isSigner: false, isWritable: true },
        { pubkey: feeToken, isSigner: false, isWritable: true },
        { pubkey: ata, isSigner: false, isWritable: true },
        { pubkey: userShares, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data: Buffer.concat([disc("withdraw"), u64(withdrawShares)]),
    }
  );
  const withdrawSig = await sendAndConfirmTransaction(connection, withdrawTx, [
    payer,
  ]);
  console.log("Withdraw:", withdrawSig);

  const cluster =
    rpc.includes("devnet") ? "devnet" : rpc.includes("127.0.0.1") || rpc.includes("localhost")
      ? "custom"
      : "devnet";
  const explorerCluster = cluster === "devnet" ? "devnet" : "custom&customUrl=" + encodeURIComponent(rpc);

  const mintInfo = await getMint(connection, mintKp.publicKey);
  const demo = {
    cluster,
    rpc,
    programId: PROGRAM_ID.toBase58(),
    authority: payer.publicKey.toBase58(),
    underlyingMint: mintKp.publicKey.toBase58(),
    vault: vault.toBase58(),
    shareMint: shareMint.toBase58(),
    vaultToken: vaultToken.toBase58(),
    feeToken: feeToken.toBase58(),
    decimals: mintInfo.decimals,
    txs: {
      createMint: mintSig,
      initialize: initSig,
      deposit: depositSig,
      withdraw: withdrawSig,
    },
    explorer: {
      program: `https://explorer.solana.com/address/${PROGRAM_ID.toBase58()}?cluster=${explorerCluster}`,
      initialize: `https://explorer.solana.com/tx/${initSig}?cluster=${explorerCluster}`,
      deposit: `https://explorer.solana.com/tx/${depositSig}?cluster=${explorerCluster}`,
      withdraw: `https://explorer.solana.com/tx/${withdrawSig}?cluster=${explorerCluster}`,
    },
  };
  fs.writeFileSync(
    path.join(root, "demo-artifacts.json"),
    JSON.stringify(demo, null, 2)
  );

  const configTs = `export const PROGRAM_ID = "${demo.programId}";

/** Filled by \`scripts/setup-devnet.mjs\` after initialize */
export const DEVNET_CONFIG = {
  cluster: "${demo.cluster === "custom" ? "localnet" : "devnet"}",
  rpcUrl: "${demo.rpc}",
  programId: PROGRAM_ID,
  underlyingMint: "${demo.underlyingMint}",
  vault: "${demo.vault}",
  shareMint: "${demo.shareMint}",
  vaultToken: "${demo.vaultToken}",
  feeToken: "${demo.feeToken}",
  authority: "${demo.authority}",
  txs: {
    initialize: "${demo.txs.initialize}",
    deposit: "${demo.txs.deposit}",
    withdraw: "${demo.txs.withdraw}",
  },
};

export const VAULT_SEED = Buffer.from("vault");
export const VAULT_TOKEN_SEED = Buffer.from("vault_token");
export const FEE_TOKEN_SEED = Buffer.from("fee_token");
export const SHARE_MINT_SEED = Buffer.from("share_mint");
`;
  fs.writeFileSync(path.join(root, "web/src/lib/config.ts"), configTs);
  console.log("Wrote demo-artifacts.json and web/src/lib/config.ts");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

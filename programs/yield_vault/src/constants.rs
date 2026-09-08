use anchor_lang::prelude::*;

/// PDA seed for the vault config account: ["vault", underlying_mint]
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

/// PDA seed for the vault's underlying token account: ["vault_token", vault]
#[constant]
pub const VAULT_TOKEN_SEED: &[u8] = b"vault_token";

/// PDA seed for the fee token account: ["fee_token", vault]
#[constant]
pub const FEE_TOKEN_SEED: &[u8] = b"fee_token";

/// PDA seed for the share mint: ["share_mint", vault]
#[constant]
pub const SHARE_MINT_SEED: &[u8] = b"share_mint";

/// Basis points denominator (100% = 10_000)
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Seconds in a year (non-leap) for APY accrual
pub const SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60;

/// Maximum APY: 100% (10_000 bps)
pub const MAX_APY_BPS: u16 = 10_000;

/// Maximum withdrawal fee: 10% (1_000 bps)
pub const MAX_WITHDRAWAL_FEE_BPS: u16 = 1_000;

/// Share mint decimals (match typical vault share precision)
pub const SHARE_DECIMALS: u8 = 6;

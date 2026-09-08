use anchor_lang::prelude::*;

/// Global vault configuration and accounting state.
///
/// Exchange rate: assets_per_share = total_assets / total_shares
/// Yield accrues by increasing `total_assets` (virtual) over time based on APY,
/// capped by tokens actually sitting in `vault_token` (must be funded for redeemable yield).
#[account]
#[derive(InitSpace)]
pub struct Vault {
    /// Admin who can fund rewards and pause (authority signer checks)
    pub authority: Pubkey,
    /// Underlying SPL / Token-2022 mint accepted by the vault
    pub underlying_mint: Pubkey,
    /// Share mint (PDA-owned mint authority = vault)
    pub share_mint: Pubkey,
    /// Token account holding underlying deposits + funded rewards
    pub vault_token: Pubkey,
    /// Token account collecting withdrawal fees (PDA, authority = vault)
    pub fee_token: Pubkey,
    /// Last snapshotted assets backing shares (principal + accrued, redeemable up to token balance)
    pub total_assets: u64,
    /// Total outstanding vault shares
    pub total_shares: u64,
    /// Annual percentage yield in basis points (e.g. 1000 = 10%)
    pub apy_bps: u16,
    /// Fee charged on withdraw in basis points
    pub withdrawal_fee_bps: u16,
    /// Minimum deposit amount (underlying raw units)
    pub min_deposit: u64,
    /// Maximum total assets (TVL cap)
    pub max_tvl: u64,
    /// Unix timestamp of last yield accrual
    pub last_update_ts: i64,
    /// When true, deposits are rejected
    pub paused: bool,
    /// Vault PDA bump
    pub bump: u8,
    /// Vault token account bump
    pub vault_token_bump: u8,
    /// Fee token account bump
    pub fee_token_bump: u8,
    /// Share mint bump
    pub share_mint_bump: u8,
}

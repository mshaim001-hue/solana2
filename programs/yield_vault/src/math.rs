//! Safe math and yield accrual helpers for the vault.

use anchor_lang::prelude::*;

use crate::constants::{BPS_DENOMINATOR, SECONDS_PER_YEAR};
use crate::error::VaultError;
use crate::state::Vault;

/// Accrue synthetic APY into `total_assets` since `last_update_ts`.
/// Redeemable yield is still limited by the vault token balance on withdraw.
pub fn accrue_yield(vault: &mut Vault, now: i64) -> Result<()> {
    if now <= vault.last_update_ts {
        return Ok(());
    }
    if vault.total_assets == 0 || vault.apy_bps == 0 {
        vault.last_update_ts = now;
        return Ok(());
    }

    let elapsed = (now - vault.last_update_ts) as u64;
    // interest = total_assets * apy_bps * elapsed / (BPS_DENOMINATOR * SECONDS_PER_YEAR)
    let interest = (vault.total_assets as u128)
        .checked_mul(vault.apy_bps as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_mul(elapsed as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(SECONDS_PER_YEAR as u128)
        .ok_or(VaultError::MathOverflow)?;

    let interest_u64 = u64::try_from(interest).map_err(|_| VaultError::MathOverflow)?;
    vault.total_assets = vault
        .total_assets
        .checked_add(interest_u64)
        .ok_or(VaultError::MathOverflow)?;
    vault.last_update_ts = now;
    Ok(())
}

/// Convert underlying amount → shares minted.
pub fn shares_for_deposit(vault: &Vault, amount: u64) -> Result<u64> {
    if amount == 0 {
        return err!(VaultError::ZeroAmount);
    }
    if vault.total_shares == 0 || vault.total_assets == 0 {
        return Ok(amount);
    }
    let shares = (amount as u128)
        .checked_mul(vault.total_shares as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(vault.total_assets as u128)
        .ok_or(VaultError::MathOverflow)?;
    let shares_u64 = u64::try_from(shares).map_err(|_| VaultError::MathOverflow)?;
    require!(shares_u64 > 0, VaultError::ZeroShares);
    Ok(shares_u64)
}

/// Convert shares → underlying assets (gross, before fee).
pub fn assets_for_shares(vault: &Vault, shares: u64) -> Result<u64> {
    if shares == 0 {
        return err!(VaultError::ZeroAmount);
    }
    require!(shares <= vault.total_shares, VaultError::InsufficientShares);
    if vault.total_shares == 0 {
        return Ok(0);
    }
    let assets = (shares as u128)
        .checked_mul(vault.total_assets as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(vault.total_shares as u128)
        .ok_or(VaultError::MathOverflow)?;
    u64::try_from(assets).map_err(|_| VaultError::MathOverflow.into())
}

/// Split gross assets into (user_receives, fee).
pub fn apply_withdrawal_fee(gross: u64, fee_bps: u16) -> Result<(u64, u64)> {
    let fee = (gross as u128)
        .checked_mul(fee_bps as u128)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(VaultError::MathOverflow)?;
    let fee_u64 = u64::try_from(fee).map_err(|_| VaultError::MathOverflow)?;
    let net = gross.checked_sub(fee_u64).ok_or(VaultError::MathOverflow)?;
    Ok((net, fee_u64))
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn sample_vault(assets: u64, shares: u64, apy_bps: u16) -> Vault {
        Vault {
            authority: Pubkey::default(),
            underlying_mint: Pubkey::default(),
            share_mint: Pubkey::default(),
            vault_token: Pubkey::default(),
            fee_token: Pubkey::default(),
            total_assets: assets,
            total_shares: shares,
            apy_bps,
            withdrawal_fee_bps: 50,
            min_deposit: 1,
            max_tvl: u64::MAX,
            last_update_ts: 1_000_000,
            paused: false,
            bump: 255,
            vault_token_bump: 255,
            fee_token_bump: 255,
            share_mint_bump: 255,
        }
    }

    #[test]
    fn first_deposit_is_one_to_one() {
        let vault = sample_vault(0, 0, 1000);
        assert_eq!(shares_for_deposit(&vault, 1_000_000).unwrap(), 1_000_000);
    }

    #[test]
    fn proportional_shares_after_yield() {
        // 2m assets / 1m shares => depositing 1m should mint 500k shares
        let vault = sample_vault(2_000_000, 1_000_000, 1000);
        assert_eq!(shares_for_deposit(&vault, 1_000_000).unwrap(), 500_000);
    }

    #[test]
    fn withdrawal_fee_split() {
        let (net, fee) = apply_withdrawal_fee(1_000_000, 50).unwrap(); // 0.5%
        assert_eq!(fee, 5_000);
        assert_eq!(net, 995_000);
    }

    #[test]
    fn accrue_increases_assets() {
        let mut vault = sample_vault(1_000_000_000, 1_000_000_000, 10_000); // 100% APY
        // Accrue for half a year
        accrue_yield(&mut vault, 1_000_000 + (SECONDS_PER_YEAR / 2) as i64).unwrap();
        assert!(vault.total_assets > 1_000_000_000);
        // Roughly +50% at 100% APY over half year
        assert!(vault.total_assets >= 1_499_000_000 && vault.total_assets <= 1_501_000_000);
    }

    #[test]
    fn zero_deposit_errors() {
        let vault = sample_vault(0, 0, 1000);
        assert!(shares_for_deposit(&vault, 0).is_err());
    }
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::error::VaultError;
use crate::state::Vault;

#[derive(Accounts)]
#[instruction(apy_bps: u16, withdrawal_fee_bps: u16, min_deposit: u64, max_tvl: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub underlying_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = 8 + Vault::INIT_SPACE,
        seeds = [VAULT_SEED, underlying_mint.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        payer = authority,
        seeds = [SHARE_MINT_SEED, vault.key().as_ref()],
        bump,
        mint::decimals = SHARE_DECIMALS,
        mint::authority = vault,
        mint::token_program = token_program,
    )]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = authority,
        seeds = [VAULT_TOKEN_SEED, vault.key().as_ref()],
        bump,
        token::mint = underlying_mint,
        token::authority = vault,
        token::token_program = token_program,
    )]
    pub vault_token: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        seeds = [FEE_TOKEN_SEED, vault.key().as_ref()],
        bump,
        token::mint = underlying_mint,
        token::authority = vault,
        token::token_program = token_program,
    )]
    pub fee_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(
    ctx: Context<Initialize>,
    apy_bps: u16,
    withdrawal_fee_bps: u16,
    min_deposit: u64,
    max_tvl: u64,
) -> Result<()> {
    require!(apy_bps <= MAX_APY_BPS, VaultError::InvalidApy);
    require!(
        withdrawal_fee_bps <= MAX_WITHDRAWAL_FEE_BPS,
        VaultError::InvalidFee
    );
    require!(min_deposit > 0, VaultError::ZeroAmount);
    require!(max_tvl >= min_deposit, VaultError::InvalidLimits);

    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;
    vault.authority = ctx.accounts.authority.key();
    vault.underlying_mint = ctx.accounts.underlying_mint.key();
    vault.share_mint = ctx.accounts.share_mint.key();
    vault.vault_token = ctx.accounts.vault_token.key();
    vault.fee_token = ctx.accounts.fee_token.key();
    vault.total_assets = 0;
    vault.total_shares = 0;
    vault.apy_bps = apy_bps;
    vault.withdrawal_fee_bps = withdrawal_fee_bps;
    vault.min_deposit = min_deposit;
    vault.max_tvl = max_tvl;
    vault.last_update_ts = clock.unix_timestamp;
    vault.paused = false;
    vault.bump = ctx.bumps.vault;
    vault.vault_token_bump = ctx.bumps.vault_token;
    vault.fee_token_bump = ctx.bumps.fee_token;
    vault.share_mint_bump = ctx.bumps.share_mint;

    msg!(
        "Vault initialized: apy={}bps fee={}bps min={} max_tvl={}",
        apy_bps,
        withdrawal_fee_bps,
        min_deposit,
        max_tvl
    );
    Ok(())
}

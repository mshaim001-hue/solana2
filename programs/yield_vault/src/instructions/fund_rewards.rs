use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::constants::*;
use crate::error::VaultError;
use crate::math::accrue_yield;
use crate::state::Vault;

/// Authority deposits extra underlying tokens so accrued yield is redeemable.
#[derive(Accounts)]
pub struct FundRewards<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, vault.underlying_mint.as_ref()],
        bump = vault.bump,
        has_one = authority @ VaultError::Unauthorized,
        has_one = underlying_mint @ VaultError::InvalidMint,
        has_one = vault_token @ VaultError::InvalidTokenAccount,
    )]
    pub vault: Account<'info, Vault>,

    pub underlying_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_TOKEN_SEED, vault.key().as_ref()],
        bump = vault.vault_token_bump,
        token::mint = underlying_mint,
        token::authority = vault,
    )]
    pub vault_token: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = authority_underlying.mint == vault.underlying_mint @ VaultError::InvalidMint,
        constraint = authority_underlying.owner == authority.key() @ VaultError::InvalidTokenAccount,
    )]
    pub authority_underlying: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_fund_rewards(ctx: Context<FundRewards>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroAmount);

    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    accrue_yield(vault, clock.unix_timestamp)?;

    let decimals = ctx.accounts.underlying_mint.decimals;
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.authority_underlying.to_account_info(),
                mint: ctx.accounts.underlying_mint.to_account_info(),
                to: ctx.accounts.vault_token.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount,
        decimals,
    )?;

    // Liquidity increases; accounting assets already include accrued virtual yield.
    // Funding does not change share supply — it backs redeemable yield.
    msg!("Funded vault with {} reward tokens", amount);
    Ok(())
}

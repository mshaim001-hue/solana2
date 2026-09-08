use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::*;
use crate::error::VaultError;
use crate::math::{accrue_yield, shares_for_deposit};
use crate::state::Vault;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, vault.underlying_mint.as_ref()],
        bump = vault.bump,
        has_one = underlying_mint @ VaultError::InvalidMint,
        has_one = share_mint @ VaultError::InvalidMint,
        has_one = vault_token @ VaultError::InvalidTokenAccount,
    )]
    pub vault: Box<Account<'info, Vault>>,

    pub underlying_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [SHARE_MINT_SEED, vault.key().as_ref()],
        bump = vault.share_mint_bump,
        mint::authority = vault,
    )]
    pub share_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [VAULT_TOKEN_SEED, vault.key().as_ref()],
        bump = vault.vault_token_bump,
        token::mint = underlying_mint,
        token::authority = vault,
    )]
    pub vault_token: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = user_underlying.mint == vault.underlying_mint @ VaultError::InvalidMint,
        constraint = user_underlying.owner == user.key() @ VaultError::InvalidTokenAccount,
    )]
    pub user_underlying: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = user_shares.mint == vault.share_mint @ VaultError::InvalidMint,
        constraint = user_shares.owner == user.key() @ VaultError::InvalidTokenAccount,
    )]
    pub user_shares: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroAmount);

    let vault = &mut ctx.accounts.vault;
    require!(!vault.paused, VaultError::VaultPaused);
    require!(amount >= vault.min_deposit, VaultError::BelowMinDeposit);

    let clock = Clock::get()?;
    accrue_yield(vault, clock.unix_timestamp)?;

    let new_tvl = vault
        .total_assets
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;
    require!(new_tvl <= vault.max_tvl, VaultError::ExceedsMaxTvl);

    let shares = shares_for_deposit(vault, amount)?;

    // Transfer underlying from user → vault
    let decimals = ctx.accounts.underlying_mint.decimals;
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.user_underlying.to_account_info(),
                mint: ctx.accounts.underlying_mint.to_account_info(),
                to: ctx.accounts.vault_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
        decimals,
    )?;

    // Mint shares to user (vault PDA is mint authority)
    let mint_key = vault.underlying_mint;
    let seeds: &[&[u8]] = &[VAULT_SEED, mint_key.as_ref(), &[vault.bump]];
    let signer = &[seeds];

    token_interface::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintTo {
                mint: ctx.accounts.share_mint.to_account_info(),
                to: ctx.accounts.user_shares.to_account_info(),
                authority: vault.to_account_info(),
            },
            signer,
        ),
        shares,
    )?;

    vault.total_assets = new_tvl;
    vault.total_shares = vault
        .total_shares
        .checked_add(shares)
        .ok_or(VaultError::MathOverflow)?;

    msg!("Deposited {} underlying for {} shares", amount, shares);
    Ok(())
}

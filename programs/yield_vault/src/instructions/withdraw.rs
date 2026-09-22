use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::*;
use crate::error::VaultError;
use crate::math::{accrue_yield, apply_withdrawal_fee, assets_for_shares};
use crate::state::Vault;

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

    /// ATA of `vault.fee_recipient`.
    /// Pass `None` (program id placeholder) when fee_recipient == user to avoid
    /// duplicate mutable account (same ATA cannot appear twice as writable).
    #[account(
        mut,
        constraint = fee_recipient_ata.mint == vault.underlying_mint @ VaultError::InvalidMint,
        constraint = fee_recipient_ata.owner == vault.fee_recipient @ VaultError::InvalidTokenAccount,
        constraint = fee_recipient_ata.key() != user_underlying.key() @ VaultError::InvalidTokenAccount,
    )]
    pub fee_recipient_ata: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

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

pub fn handle_withdraw(ctx: Context<Withdraw>, shares: u64, min_out: u64) -> Result<()> {
    require!(shares > 0, VaultError::ZeroAmount);
    require!(
        ctx.accounts.user_shares.amount >= shares,
        VaultError::InsufficientShares
    );

    let fee_recipient = ctx.accounts.vault.fee_recipient;
    let user_is_fee_recipient = fee_recipient == ctx.accounts.user.key();

    match (
        user_is_fee_recipient,
        ctx.accounts.fee_recipient_ata.is_some(),
    ) {
        // Same wallet: must omit fee ATA (avoid duplicate mutable account).
        (true, true) => return err!(VaultError::InvalidTokenAccount),
        // Different wallet: fee ATA is required.
        (false, false) => return err!(VaultError::InvalidTokenAccount),
        _ => {}
    }

    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    accrue_yield(vault, clock.unix_timestamp)?;

    let gross = assets_for_shares(vault, shares)?;
    require!(gross > 0, VaultError::NothingToWithdraw);

    let (net, fee) = apply_withdrawal_fee(gross, vault.withdrawal_fee_bps)?;
    require!(net >= min_out, VaultError::SlippageExceeded);

    let total_out = net
        .checked_add(fee)
        .ok_or(VaultError::MathOverflow)?;
    require!(
        ctx.accounts.vault_token.amount >= total_out,
        VaultError::InsufficientLiquidity
    );

    token_interface::burn(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Burn {
                mint: ctx.accounts.share_mint.to_account_info(),
                from: ctx.accounts.user_shares.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        shares,
    )?;

    let mint_key = vault.underlying_mint;
    let bump = vault.bump;
    let seeds: &[&[u8]] = &[VAULT_SEED, mint_key.as_ref(), &[bump]];
    let signer = &[seeds];
    let decimals = ctx.accounts.underlying_mint.decimals;
    let token_program = ctx.accounts.token_program.key();

    if user_is_fee_recipient {
        // One transfer: user receives net + fee in the same ATA.
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                token_program,
                TransferChecked {
                    from: ctx.accounts.vault_token.to_account_info(),
                    mint: ctx.accounts.underlying_mint.to_account_info(),
                    to: ctx.accounts.user_underlying.to_account_info(),
                    authority: vault.to_account_info(),
                },
                signer,
            ),
            total_out,
            decimals,
        )?;
    } else {
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                token_program,
                TransferChecked {
                    from: ctx.accounts.vault_token.to_account_info(),
                    mint: ctx.accounts.underlying_mint.to_account_info(),
                    to: ctx.accounts.user_underlying.to_account_info(),
                    authority: vault.to_account_info(),
                },
                signer,
            ),
            net,
            decimals,
        )?;

        if fee > 0 {
            let fee_ata = ctx
                .accounts
                .fee_recipient_ata
                .as_ref()
                .ok_or(VaultError::InvalidTokenAccount)?;
            token_interface::transfer_checked(
                CpiContext::new_with_signer(
                    token_program,
                    TransferChecked {
                        from: ctx.accounts.vault_token.to_account_info(),
                        mint: ctx.accounts.underlying_mint.to_account_info(),
                        to: fee_ata.to_account_info(),
                        authority: vault.to_account_info(),
                    },
                    signer,
                ),
                fee,
                decimals,
            )?;
        }
    }

    vault.total_shares = vault
        .total_shares
        .checked_sub(shares)
        .ok_or(VaultError::MathOverflow)?;
    vault.total_assets = vault
        .total_assets
        .checked_sub(gross)
        .ok_or(VaultError::MathOverflow)?;

    msg!(
        "Withdrew {} shares → {} net + {} fee (gross {}, min_out {})",
        shares,
        net,
        fee,
        gross,
        min_out
    );
    Ok(())
}

use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::VaultError;
use crate::state::Vault;

#[derive(Accounts)]
pub struct SetPaused<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, vault.underlying_mint.as_ref()],
        bump = vault.bump,
        has_one = authority @ VaultError::Unauthorized,
    )]
    pub vault: Account<'info, Vault>,
}

pub fn handle_set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
    ctx.accounts.vault.paused = paused;
    msg!("Vault paused = {}", paused);
    Ok(())
}

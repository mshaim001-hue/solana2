pub mod constants;
pub mod error;
pub mod instructions;
pub mod math;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("9gf1uFbnaP1LZymvDWmCW92aW7upE8KGvWudwdprq7hu");

#[program]
pub mod yield_vault {
    use super::*;

    /// Create vault PDA, share mint, and vault token account.
    pub fn initialize(
        ctx: Context<Initialize>,
        apy_bps: u16,
        withdrawal_fee_bps: u16,
        min_deposit: u64,
        max_tvl: u64,
    ) -> Result<()> {
        instructions::initialize::handle_initialize(
            ctx,
            apy_bps,
            withdrawal_fee_bps,
            min_deposit,
            max_tvl,
        )
    }

    /// Deposit underlying SPL tokens and receive vault shares.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handle_deposit(ctx, amount)
    }

    /// Burn shares and withdraw underlying (minus withdrawal fee).
    pub fn withdraw(ctx: Context<Withdraw>, shares: u64) -> Result<()> {
        instructions::withdraw::handle_withdraw(ctx, shares)
    }

    /// Authority tops up vault liquidity so accrued yield can be redeemed.
    pub fn fund_rewards(ctx: Context<FundRewards>, amount: u64) -> Result<()> {
        instructions::fund_rewards::handle_fund_rewards(ctx, amount)
    }

    /// Pause / unpause deposits.
    pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
        instructions::set_paused::handle_set_paused(ctx, paused)
    }
}

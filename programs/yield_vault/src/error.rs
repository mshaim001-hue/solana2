use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Only the vault authority can perform this action")]
    Unauthorized,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Deposit below minimum")]
    BelowMinDeposit,
    #[msg("Deposit would exceed max TVL")]
    ExceedsMaxTvl,
    #[msg("Vault is paused")]
    VaultPaused,
    #[msg("Insufficient vault shares")]
    InsufficientShares,
    #[msg("Insufficient vault liquidity for withdrawal")]
    InsufficientLiquidity,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid APY (max 100%)")]
    InvalidApy,
    #[msg("Invalid withdrawal fee (max 10%)")]
    InvalidFee,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Invalid token account owner or mint")]
    InvalidTokenAccount,
    #[msg("Max TVL must be >= min deposit")]
    InvalidLimits,
    #[msg("Nothing to withdraw for the given shares")]
    NothingToWithdraw,
    #[msg("Share calculation produced zero shares")]
    ZeroShares,
}

use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Legacy vault has insufficient withdrawable funds")]
    InsufficientFunds,
    #[msg("The receipt mint must not have a freeze authority")]
    FreezeAuthorityNotRevoked,
    #[msg("Vault receipt balance must be nonzero while shares are outstanding")]
    EmptyVaultWithOutstandingShares,
    #[msg("The calculated share or receipt amount rounds to zero")]
    ZeroValue,
    #[msg("The calculated amount is below the caller's minimum")]
    SlippageExceeded,
    #[msg("Arithmetic overflow or conversion failure")]
    MathOverflow,
}

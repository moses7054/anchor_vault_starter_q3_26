use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Deposit amount must be greater than zero")]
    InvalidAmount,
    #[msg("Insufficient withdrawable balance in the vault")]
    InsufficientFunds,
}

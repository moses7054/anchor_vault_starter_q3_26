pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("FEV3t9iJMwDb2wYFHiaESXoQuDRT3wWuSR8NiQa6bdXS");

#[program]
pub mod q3_26_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, min_shares_out: u64) -> Result<()> {
        ctx.accounts.deposit(amount, min_shares_out)
    }

    pub fn redeem(ctx: Context<Redeem>, shares: u64, min_receipts_out: u64) -> Result<()> {
        ctx.accounts.redeem(shares, min_receipts_out)
    }

    pub fn withdraw(ctx: Context<LegacyWithdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn close(ctx: Context<LegacyClose>) -> Result<()> {
        ctx.accounts.close()
    }
}

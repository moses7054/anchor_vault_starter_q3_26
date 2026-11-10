use crate::{
    constants::{STATE, VAULT_SEED},
    error::ErrorCode,
    state::VaultState,
};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct LegacyWithdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [STATE, user.key().as_ref()],
        bump = vault_state.state_bump
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [VAULT_SEED, user.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> LegacyWithdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let remaining = self
            .vault
            .lamports()
            .checked_sub(amount)
            .ok_or(ErrorCode::InsufficientFunds)?;
        let rent_minimum = Rent::get()?.minimum_balance(self.vault.data_len());
        require!(remaining >= rent_minimum, ErrorCode::InsufficientFunds);

        let user_key = self.user.key();
        let bump = [self.vault_state.vault_bump];
        let seeds = &[VAULT_SEED, user_key.as_ref(), &bump[..]];
        let signer_seeds = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        transfer(
            CpiContext::new_with_signer(self.system_program.key(), cpi_accounts, signer_seeds),
            amount,
        )
    }
}

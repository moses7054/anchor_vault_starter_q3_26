

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
pub struct Withdraw<'info> {
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

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        // Reject zero-value withdrawals.
        require!(amount > 0, ErrorCode::InvalidAmount);

        // Preserve the vault's rent-exempt minimum balance.
        let rent_reserve =
            Rent::get()?.minimum_balance(self.vault.data_len());

        let vault_balance = self.vault.lamports();

        let withdrawable = vault_balance
            .checked_sub(rent_reserve)
            .unwrap_or(0);

        // Prevent withdrawing more than the available balance.
        require!(
            amount <= withdrawable,
            ErrorCode::InsufficientFunds
        );

        // PDA signer seeds authorize the vault to transfer SOL.
        let user_key = self.user.key();
        let bump = [self.vault_state.vault_bump];

        let signer_seeds: &[&[u8]] = &[
            VAULT_SEED,
            user_key.as_ref(),
            &bump,
        ];

        let signer = &[signer_seeds];

        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.system_program.key(),
            cpi_accounts,
            signer,
        );

        transfer(cpi_ctx, amount)
    }
}

use crate::{
    constants::{STATE, VAULT_SEED},
    state::VaultState,
};

use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        close = user,
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

impl<'info> Close<'info> {
    pub fn close(&mut self) -> Result<()> {
        let vault_balance = self.vault.lamports();

        if vault_balance > 0 {
            let user_key = self.user.key();
            let bump = [self.vault_state.vault_bump];

            let signer_seeds: &[&[u8]] = &[VAULT_SEED, user_key.as_ref(), &bump];

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

            transfer(cpi_ctx, vault_balance)?;
        }

        // Anchor closes vault_state and returns its rent to user.
        Ok(())
    }
}

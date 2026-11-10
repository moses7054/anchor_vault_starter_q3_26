use crate::{
    constants::{RECEIPT_VAULT_STATE, SHARE_MINT_SEED, VAULT_SEED},
    error::ErrorCode,
    state::ReceiptVaultState,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [RECEIPT_VAULT_STATE, receipt_mint.key().as_ref()],
        bump = vault_state.state_bump,
        has_one = receipt_mint,
        has_one = share_mint
    )]
    pub vault_state: Account<'info, ReceiptVaultState>,

    pub receipt_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_SEED, receipt_mint.key().as_ref()],
        bump = vault_state.vault_bump,
        token::mint = receipt_mint,
        token::authority = vault_state
    )]
    pub receipt_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [SHARE_MINT_SEED, receipt_mint.key().as_ref()],
        bump = vault_state.share_mint_bump,
        address = vault_state.share_mint
    )]
    pub share_mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = share_mint,
        token::authority = user
    )]
    pub user_share_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = receipt_mint,
        token::authority = user
    )]
    pub user_receipt_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Redeem<'info> {
    pub fn redeem(&mut self, shares: u64, min_receipts_out: u64) -> Result<()> {
        require!(shares > 0, ErrorCode::InvalidAmount);
        require!(self.share_mint.supply > 0, ErrorCode::ZeroValue);
        require!(
            self.receipt_vault.amount > 0,
            ErrorCode::EmptyVaultWithOutstandingShares
        );
        require!(shares <= self.share_mint.supply, ErrorCode::InvalidAmount);

        let receipts_out = u64::try_from(
            (shares as u128)
                .checked_mul(self.receipt_vault.amount as u128)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(self.share_mint.supply as u128)
                .ok_or(ErrorCode::MathOverflow)?,
        )
        .map_err(|_| error!(ErrorCode::MathOverflow))?;
        require!(receipts_out > 0, ErrorCode::ZeroValue);
        require!(
            receipts_out >= min_receipts_out,
            ErrorCode::SlippageExceeded
        );

        token::burn(
            CpiContext::new(
                self.token_program.key(),
                Burn {
                    mint: self.share_mint.to_account_info(),
                    from: self.user_share_account.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            shares,
        )?;

        let receipt_mint_key = self.receipt_mint.key();
        let bump = [self.vault_state.state_bump];
        let state_seeds = &[RECEIPT_VAULT_STATE, receipt_mint_key.as_ref(), &bump[..]];
        let signer_seeds = &[&state_seeds[..]];
        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from: self.receipt_vault.to_account_info(),
                    to: self.user_receipt_account.to_account_info(),
                    authority: self.vault_state.to_account_info(),
                },
                signer_seeds,
            ),
            receipts_out,
        )?;

        Ok(())
    }
}

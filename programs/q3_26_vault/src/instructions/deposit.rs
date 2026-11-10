use crate::{
    constants::{RECEIPT_VAULT_STATE, SHARE_MINT_SEED, VAULT_SEED},
    error::ErrorCode,
    state::ReceiptVaultState,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        token::mint = receipt_mint,
        token::authority = user
    )]
    pub user_receipt_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = share_mint,
        token::authority = user
    )]
    pub user_share_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64, min_shares_out: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        let balance_before = self.receipt_vault.amount;
        let supply_before = self.share_mint.supply;
        if supply_before > 0 {
            require!(
                balance_before > 0,
                ErrorCode::EmptyVaultWithOutstandingShares
            );
        }

        token::transfer(
            CpiContext::new(
                self.token_program.key(),
                Transfer {
                    from: self.user_receipt_account.to_account_info(),
                    to: self.receipt_vault.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
        )?;

        self.receipt_vault.reload()?;
        let balance_after = self.receipt_vault.amount;
        let received = balance_after
            .checked_sub(balance_before)
            .ok_or(ErrorCode::MathOverflow)?;
        let shares_out = if supply_before == 0 {
            balance_after
        } else {
            u64::try_from(
                (received as u128)
                    .checked_mul(supply_before as u128)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(balance_before as u128)
                    .ok_or(ErrorCode::MathOverflow)?,
            )
            .map_err(|_| error!(ErrorCode::MathOverflow))?
        };

        require!(shares_out > 0, ErrorCode::ZeroValue);
        require!(shares_out >= min_shares_out, ErrorCode::SlippageExceeded);

        let receipt_mint_key = self.receipt_mint.key();
        let bump = [self.vault_state.state_bump];
        let state_seeds = &[RECEIPT_VAULT_STATE, receipt_mint_key.as_ref(), &bump[..]];
        let signer_seeds = &[&state_seeds[..]];
        token::mint_to(
            CpiContext::new_with_signer(
                self.token_program.key(),
                MintTo {
                    mint: self.share_mint.to_account_info(),
                    to: self.user_share_account.to_account_info(),
                    authority: self.vault_state.to_account_info(),
                },
                signer_seeds,
            ),
            shares_out,
        )?;

        Ok(())
    }
}

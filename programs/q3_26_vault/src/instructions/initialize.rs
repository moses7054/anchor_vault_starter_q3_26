use crate::{
    constants::{RECEIPT_VAULT_STATE, SHARE_MINT_SEED, VAULT_SEED},
    error::ErrorCode,
    state::ReceiptVaultState,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub receipt_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        seeds = [RECEIPT_VAULT_STATE, receipt_mint.key().as_ref()],
        bump,
        space = ReceiptVaultState::DISCRIMINATOR.len() + ReceiptVaultState::INIT_SPACE
    )]
    pub vault_state: Account<'info, ReceiptVaultState>,

    #[account(
        init,
        payer = payer,
        seeds = [VAULT_SEED, receipt_mint.key().as_ref()],
        bump,
        token::mint = receipt_mint,
        token::authority = vault_state
    )]
    pub receipt_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        seeds = [SHARE_MINT_SEED, receipt_mint.key().as_ref()],
        bump,
        mint::decimals = receipt_mint.decimals,
        mint::authority = vault_state
    )]
    pub share_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        require!(
            self.receipt_mint.freeze_authority.is_none(),
            ErrorCode::FreezeAuthorityNotRevoked
        );

        self.vault_state.receipt_mint = self.receipt_mint.key();
        self.vault_state.share_mint = self.share_mint.key();
        self.vault_state.vault_bump = bumps.receipt_vault;
        self.vault_state.share_mint_bump = bumps.share_mint;
        self.vault_state.state_bump = bumps.vault_state;
        Ok(())
    }
}

use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ReceiptVaultState {
    pub receipt_mint: Pubkey,
    pub share_mint: Pubkey,
    pub vault_bump: u8,
    pub share_mint_bump: u8,
    pub state_bump: u8,
}

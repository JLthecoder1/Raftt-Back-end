use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]

pub struct Offering {
    pub authority: Pubkey,
    pub mint: Pubkey,        // Novo: Token aceito (dólar SPL)
    pub target_amount: u64,
    pub raised_amount: u64,
    pub is_closed: bool,     // Novo: Indica se a oferta fechou
    pub vault: Pubkey,       // Novo: Endereço do cofre PDA
    pub bump: u8,
}
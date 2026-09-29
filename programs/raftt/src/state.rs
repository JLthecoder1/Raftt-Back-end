use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]

pub struct Offering {
    pub authority: Pubkey,   // Quem criou a oferta (o emissor)
    pub target_amount: u64,  // A meta de captação desejada
    pub raised_amount: u64,  // O quanto já foi investido até ao momento
    pub bump: u8,            // O identificador de segurança da conta (bump seed)
}
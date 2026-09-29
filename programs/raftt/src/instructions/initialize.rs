use anchor_lang::prelude::*;
use crate::{error::ErrorCode, state::Offering};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    // Cria a conta da Oferta (Offering) na blockchain
    #[account(
        init,
        payer = user,
        space = 8 + Offering::INIT_SPACE,
        seeds = [b"offering", user.key().as_ref()],
        bump
    )]
    pub offering: Account<'info, Offering>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>, target_amount: u64) -> Result<()> {
    require!(target_amount > 0, ErrorCode::InvalidTargetAmount);

    let offering = &mut ctx.accounts.offering;
    
    // Preenche os dados iniciais da oferta da Raftt
    offering.authority = ctx.accounts.user.key();
    offering.target_amount = target_amount;
    offering.raised_amount = 0; // Começa com zero arrecadado
    offering.bump = ctx.bumps.offering;

    msg!("Oferta da Rafft inicializada com sucesso! Meta: {}", target_amount);
    Ok(())
}
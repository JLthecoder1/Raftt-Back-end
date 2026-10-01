use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::{error::ErrorCode, state::Offering};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// Mint do token SPL (estável em dólar) aceito na oferta
    pub mint: Account<'info, Mint>,

    // Cria a conta da Oferta (Offering) na blockchain
    #[account(
        init,
        payer = user,
        space = 8 + Offering::INIT_SPACE,
        seeds = [b"offering", user.key().as_ref()],
        bump
    )]
    pub offering: Account<'info, Offering>,

    /// Cofre PDA para reter os fundos arrecadados em tokens SPL
    #[account(
        init,
        payer = user,
        token::mint = mint,
        token::authority = offering,
        seeds = [b"vault", offering.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_initialize(ctx: Context<Initialize>, target_amount: u64) -> Result<()> {
    require!(target_amount > 0, ErrorCode::InvalidTargetAmount);

    let offering = &mut ctx.accounts.offering;
    
    // Preenche os dados iniciais da oferta da Raftt com os novos campos
    offering.authority = ctx.accounts.user.key();
    offering.mint = ctx.accounts.mint.key();
    offering.target_amount = target_amount;
    offering.raised_amount = 0;
    offering.is_closed = false;
    offering.vault = ctx.accounts.vault.key();
    offering.bump = ctx.bumps.offering;

    msg!("Oferta da Raftt inicializada com sucesso! Meta: {}", target_amount);
    Ok(())
}
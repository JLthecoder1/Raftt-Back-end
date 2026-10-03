use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, TransferChecked};
use crate::{
    error::ErrorCode,
    state::{Offering, Position},
};

#[derive(Accounts)]
pub struct Invest<'info> {
    #[account(mut)]
    pub investor: Signer<'info>,

    /// Conta de tokens SPL do investidor
    #[account(
        mut,
        constraint = investor_token_account.mint == offering.mint,
        constraint = investor_token_account.owner == investor.key()
    )]
    pub investor_token_account: Account<'info, TokenAccount>,

    /// Mint do token aceito na oferta
    #[account(
        address = offering.mint
    )]
    pub mint: Account<'info, Mint>,

    /// A conta da Oferta
    #[account(
        mut,
        seeds = [b"offering", offering.authority.as_ref()],
        bump = offering.bump,
        constraint = !offering.is_closed @ ErrorCode::OfferingIsClosed
    )]
    pub offering: Account<'info, Offering>,

    #[account(
        init_if_needed,
        payer = investor,
        space = 8 + Position::INIT_SPACE,
        seeds = [b"position", offering.key().as_ref(), investor.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    /// O cofre PDA para onde os tokens vão
    #[account(
        mut,
        address = offering.vault,
        constraint = vault.mint == offering.mint,
        constraint = vault.owner == offering.key(),
    )]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    
}

pub fn handle_invest(ctx: Context<Invest>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::InvalidInvestmentAmount);

    let offering = &mut ctx.accounts.offering;

    // Garante que o novo montante não ultrapassa a meta
    let new_raised = offering.raised_amount
        .checked_add(amount)
        .ok_or(ErrorCode::NumericalOverflow)?;

    require!(new_raised <= offering.target_amount, ErrorCode::ExceedsTargetAmount);

    // Transfere os tokens do investidor para o cofre PDA usando TransferChecked
    let cpi_accounts = TransferChecked {
        from: ctx.accounts.investor_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.investor.to_account_info(),
    };
    
    // Passa a Pubkey do token_program usando .key()
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        cpi_accounts,
    );

    // Usa as decimais corretas da conta Mint
    token::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    let position = &mut ctx.accounts.position;

    position.offering = offering.key();
    position.investor = ctx.accounts.investor.key();

    position.amount = position
        .amount
        .checked_add(amount)
        .ok_or(ErrorCode::NumericalOverflow)?;

    position.bump = ctx.bumps.position;
    // Atualiza o estado da oferta
    offering.raised_amount = new_raised;

    // Se atingiu a meta exata, fecha a oferta automaticamente
    if offering.raised_amount == offering.target_amount {
        offering.is_closed = true;
    }

    msg!("Investimento de {} tokens realizado com sucesso! Total arrecadado: {}", amount, offering.raised_amount);
    Ok(())
}
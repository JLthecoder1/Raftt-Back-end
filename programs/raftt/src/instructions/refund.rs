use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    error::ErrorCode,
    state::{Offering, Position},
};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub investor: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"offering",
            offering.authority.as_ref()
        ],
        bump = offering.bump,
        constraint = offering.is_failed @ ErrorCode::OfferingNotFailed
    )]
    pub offering: Account<'info, Offering>,

    #[account(
        mut,
        seeds = [
            b"position",
            offering.key().as_ref(),
            investor.key().as_ref()
        ],
        bump = position.bump,
        constraint = position.investor == investor.key()
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        address = offering.vault,
        constraint = vault.mint == offering.mint,
        constraint = vault.owner == offering.key(),
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = investor_token_account.mint == offering.mint,
        constraint = investor_token_account.owner == investor.key()
    )]
    pub investor_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_refund(ctx: Context<Refund>) -> Result<()> {
    let position = &mut ctx.accounts.position;

    require!(position.amount > 0, ErrorCode::InvalidInvestmentAmount);

    let amount = position.amount;

    let seeds = &[
        b"offering",
        ctx.accounts.offering.authority.as_ref(),
        &[ctx.accounts.offering.bump],
    ];

    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = anchor_spl::token::Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.investor_token_account.to_account_info(),
        authority: ctx.accounts.offering.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        cpi_accounts,
        signer_seeds,
    );

    anchor_spl::token::transfer(cpi_ctx, amount)?;

    position.amount = 0;

    msg!("Reembolso de {} tokens realizado com sucesso!", amount);

    Ok(())
}
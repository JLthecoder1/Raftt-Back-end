use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{error::ErrorCode, state::Offering};

#[derive(Accounts)]
pub struct Release<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"offering",
            authority.key().as_ref()
        ],
        bump = offering.bump,
        constraint = offering.authority == authority.key()
    )]
    pub offering: Account<'info, Offering>,

    #[account(
        mut,
        address = offering.vault,
        constraint = vault.mint == offering.mint,
        constraint = vault.owner == offering.key()
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = destination.mint == offering.mint,
        constraint = destination.owner == authority.key()
    )]
    pub destination: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_release(ctx: Context<Release>) -> Result<()> {
    let offering = &mut ctx.accounts.offering;

    require!(
        offering.raised_amount >= offering.target_amount,
        ErrorCode::OfferingNotCompleted
    );

    require!(!offering.is_released, ErrorCode::OfferingAlreadyReleased);

    let authority_key = offering.authority;
    let bump = offering.bump;

    let seeds = &[b"offering", authority_key.as_ref(), &[bump]];

    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.destination.to_account_info(),
        authority: offering.to_account_info(),
    };

    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);

    token::transfer(cpi_ctx, offering.raised_amount)?;

    offering.is_released = true;

    msg!("Fundos liberados com sucesso.");

    Ok(())
}

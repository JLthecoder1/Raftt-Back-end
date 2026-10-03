use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    state::Offering,
};

#[derive(Accounts)]
pub struct FailOffering<'info> {
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

    pub authority: Signer<'info>,
}

pub fn handle_fail_offering(ctx: Context<FailOffering>) -> Result<()> {
    let offering = &mut ctx.accounts.offering;

    require!(!offering.is_closed, ErrorCode::OfferingIsClosed);
    require!(!offering.is_failed, ErrorCode::OfferingAlreadyFailed);

    offering.is_failed = true;

    msg!("Oferta marcada como falha.");

    Ok(())
}
pub mod constants;
pub mod v2;
pub use v2::*;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("5sSTJ3RXRcz8GmmtuSwq6epjJJQZagCCi3AaEwpffmhZ");

#[program]
pub mod raftt {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, target_amount: u64) -> Result<()> {
        crate::instructions::initialize::handle_initialize(ctx, target_amount)
    }

    pub fn invest(ctx: Context<Invest>, amount: u64) -> Result<()> {
        crate::instructions::invest::handle_invest(ctx, amount)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        crate::instructions::refund::handle_refund(ctx)
    }

    pub fn fail_offering(ctx: Context<FailOffering>) -> Result<()> {
        crate::instructions::fail_offering::handle_fail_offering(ctx)
    }

    pub fn release(ctx: Context<Release>) -> Result<()> {
        crate::instructions::release::handle_release(ctx)
    }
    pub fn setup_platform(ctx: Context<SetupPlatform>, reviewer: Pubkey) -> Result<()> { v2::setup(ctx, reviewer) }
    pub fn create_round(ctx: Context<CreateRound>, round_id: u64, minimum: u64, cap: u64, deadline: i64, recipient: Pubkey, plan_hash: [u8;32]) -> Result<()> { v2::create(ctx, round_id, minimum, cap, deadline, recipient, plan_hash) }
    pub fn add_milestone(ctx: Context<AddMilestone>, index: u16, amount: u64, plan_hash: [u8;32]) -> Result<()> { v2::add_milestone(ctx,index,amount,plan_hash) }
    pub fn open_round(ctx: Context<ReviewRound>, review_hash: [u8;32]) -> Result<()> { v2::open(ctx,review_hash) }
    pub fn fail_round(ctx: Context<ReviewRound>) -> Result<()> { v2::fail(ctx) }
    pub fn close_round(ctx: Context<CloseRound>) -> Result<()> { v2::close(ctx) }
    pub fn invest_round(ctx: Context<InvestRound>, amount: u64) -> Result<()> { v2::invest(ctx,amount) }
    pub fn cancel_round(ctx: Context<WithdrawRound>, amount: u64) -> Result<()> { v2::withdraw(ctx,amount,true) }
    pub fn refund_round(ctx: Context<WithdrawRound>) -> Result<()> { let amount=ctx.accounts.position.amount; v2::withdraw(ctx,amount,false) }
    pub fn submit_milestone(ctx: Context<ReviewMilestone>, evidence_hash: [u8;32]) -> Result<()> { v2::submit(ctx,evidence_hash) }
    pub fn approve_milestone(ctx: Context<ReviewMilestone>, review_hash: [u8;32]) -> Result<()> { v2::approve(ctx,review_hash) }
    pub fn release_milestone(ctx: Context<ReleaseMilestone>) -> Result<()> { v2::release(ctx) }
}

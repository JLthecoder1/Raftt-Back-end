//! V2 is isolated from legacy Offering accounts and their integral-release path.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

pub const CANCEL_WINDOW: i64 = 48 * 60 * 60;

#[account]
#[derive(InitSpace)]
pub struct PlatformConfig { pub authority: Pubkey, pub bump: u8 }
#[account]
#[derive(InitSpace)]
pub struct Round {
    pub creator: Pubkey,
    pub platform: Pubkey,
    pub mint: Pubkey,
    pub recipient: Pubkey,
    pub vault: Pubkey,
    pub round_id: u64,
    pub minimum: u64,
    pub cap: u64,
    pub deadline: i64,
    pub raised: u64,
    pub allocated: u64,
    pub released: u64,
    pub milestone_count: u16,
    pub next_release: u16,
    pub status: u8, // 0 draft, 1 open, 2 successful, 3 failed
    pub bump: u8,
    pub plan_hash: [u8; 32],
    pub review_hash: [u8; 32],
}
#[account]
#[derive(InitSpace)]
pub struct RoundPosition { pub round: Pubkey, pub investor: Pubkey, pub amount: u64, pub bump: u8 }
#[account]
#[derive(InitSpace)]
pub struct Milestone {
    pub round: Pubkey,
    pub index: u16,
    pub amount: u64,
    pub status: u8, // 0 planned, 1 submitted, 2 authorized, 3 paid
    pub plan_hash: [u8; 32],
    pub evidence_hash: [u8; 32],
    pub review_hash: [u8; 32],
    pub bump: u8,
}
use crate::error::ErrorCode as V2Error;

fn add(a: u64, b: u64) -> Result<u64> { a.checked_add(b).ok_or(error!(V2Error::Overflow)) }
fn hash_present(h: &[u8;32]) -> Result<()> { require!(*h != [0;32], V2Error::MissingHash); Ok(()) }
pub fn cancellation_allowed(status:u8, now:i64, deadline:i64) -> bool {
    status == 1 && deadline.checked_sub(CANCEL_WINDOW).is_some_and(|cutoff| now < cutoff)
}
pub fn closed_status(raised:u64, minimum:u64) -> u8 { if raised >= minimum {2} else {3} }

#[derive(Accounts)]
pub struct SetupPlatform<'info> {
    #[account(mut)] pub authority: Signer<'info>,
    #[account(init, payer=authority, space=8+PlatformConfig::INIT_SPACE, seeds=[b"platform-v2"], bump)]
    pub config: Account<'info, PlatformConfig>,
    #[account(constraint=program.programdata_address()? == Some(program_data.key()) @ V2Error::Unauthorized)]
    pub program: Program<'info, crate::program::Raftt>,
    #[account(constraint=program_data.upgrade_authority_address == Some(authority.key()) @ V2Error::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}
pub fn setup(ctx:Context<SetupPlatform>, reviewer:Pubkey) -> Result<()> {
    require!(reviewer != Pubkey::default(), V2Error::Unauthorized);
    ctx.accounts.config.set_inner(PlatformConfig {authority:reviewer,bump:ctx.bumps.config}); Ok(())
}
#[derive(Accounts)]
#[instruction(round_id:u64)]
pub struct CreateRound<'info> {
    #[account(mut)] pub creator: Signer<'info>,
    #[account(seeds=[b"platform-v2"], bump=config.bump)] pub config: Account<'info,PlatformConfig>,
    pub mint: Account<'info,Mint>,
    #[account(init,payer=creator,space=8+Round::INIT_SPACE,seeds=[b"round-v2",creator.key().as_ref(),&round_id.to_le_bytes()],bump)]
    pub round: Account<'info,Round>,
    #[account(init,payer=creator,token::mint=mint,token::authority=round,seeds=[b"vault-v2",round.key().as_ref()],bump)]
    pub vault: Account<'info,TokenAccount>,
    pub system_program: Program<'info,System>,
    pub token_program: Program<'info,Token>,
    pub rent: Sysvar<'info,Rent>,
}
pub fn create(ctx:Context<CreateRound>,round_id:u64,minimum:u64,cap:u64,deadline:i64,recipient:Pubkey,plan_hash:[u8;32]) -> Result<()> {
    require!(minimum>0 && cap>=minimum,V2Error::InvalidAmount);
    let now=Clock::get()?.unix_timestamp;
    require!(deadline.checked_sub(now).is_some_and(|duration| duration>CANCEL_WINDOW),V2Error::InvalidDeadline);
    require!(recipient!=Pubkey::default() && ctx.accounts.creator.key()!=ctx.accounts.config.authority,V2Error::Unauthorized);
    hash_present(&plan_hash)?;
    ctx.accounts.round.set_inner(Round {creator:ctx.accounts.creator.key(),platform:ctx.accounts.config.authority,
        mint:ctx.accounts.mint.key(),recipient,vault:ctx.accounts.vault.key(),round_id,minimum,cap,deadline,
        raised:0,allocated:0,released:0,milestone_count:0,next_release:0,status:0,bump:ctx.bumps.round,plan_hash,review_hash:[0;32]}); Ok(())
}
#[derive(Accounts)]
#[instruction(index:u16)]
pub struct AddMilestone<'info> {
    #[account(mut)] pub creator: Signer<'info>,
    #[account(mut,seeds=[b"round-v2",round.creator.as_ref(),&round.round_id.to_le_bytes()],bump=round.bump,has_one=creator)]
    pub round: Account<'info,Round>,
    #[account(init,payer=creator,space=8+Milestone::INIT_SPACE,seeds=[b"milestone-v2",round.key().as_ref(),&index.to_le_bytes()],bump)]
    pub milestone: Account<'info,Milestone>,
    pub system_program: Program<'info,System>,
}
pub fn add_milestone(ctx:Context<AddMilestone>,index:u16,amount:u64,plan_hash:[u8;32]) -> Result<()> {
    let r=&mut ctx.accounts.round;
    require!(r.status==0 || r.status==2,V2Error::InvalidState);
    require!(index==r.milestone_count,V2Error::InvalidMilestone);
    hash_present(&plan_hash)?;
    let allocated=add(r.allocated,amount)?;
    // A minimum-funded round must be able to finance the entire committed plan.
    let budget=if r.status==0 {r.minimum} else {r.raised};
    require!(amount>0 && allocated<=budget,V2Error::InvalidAmount);
    ctx.accounts.milestone.set_inner(Milestone {round:r.key(),index,amount,status:0,plan_hash,evidence_hash:[0;32],review_hash:[0;32],bump:ctx.bumps.milestone});
    r.allocated=allocated; r.milestone_count=r.milestone_count.checked_add(1).ok_or(V2Error::Overflow)?; Ok(())
}
#[derive(Accounts)]
pub struct ReviewRound<'info> {
    pub reviewer: Signer<'info>,
    #[account(mut,seeds=[b"round-v2",round.creator.as_ref(),&round.round_id.to_le_bytes()],bump=round.bump,constraint=round.platform==reviewer.key() @ V2Error::Unauthorized)]
    pub round: Account<'info,Round>,
}
pub fn open(ctx:Context<ReviewRound>,review_hash:[u8;32]) -> Result<()> {
    let r=&mut ctx.accounts.round;
    require!(r.status==0,V2Error::InvalidState);
    require!(r.allocated==r.minimum && r.milestone_count>0,V2Error::InvalidAmount);
    require!(r.deadline.checked_sub(Clock::get()?.unix_timestamp).is_some_and(|duration|duration>CANCEL_WINDOW),V2Error::InvalidDeadline);
    hash_present(&review_hash)?; r.review_hash=review_hash; r.status=1; Ok(())
}
pub fn fail(ctx:Context<ReviewRound>) -> Result<()> {
    require!(ctx.accounts.round.status<=1,V2Error::InvalidState);
    ctx.accounts.round.status=3; Ok(())
}
#[derive(Accounts)]
pub struct CloseRound<'info> {
    #[account(mut,seeds=[b"round-v2",round.creator.as_ref(),&round.round_id.to_le_bytes()],bump=round.bump)]
    pub round: Account<'info,Round>,
}
pub fn close(ctx:Context<CloseRound>) -> Result<()> {
    let r=&mut ctx.accounts.round;
    require!(r.status==1,V2Error::InvalidState);
    require!(Clock::get()?.unix_timestamp>=r.deadline,V2Error::InvalidDeadline);
    r.status=closed_status(r.raised,r.minimum); Ok(())
}
#[derive(Accounts)]
pub struct InvestRound<'info> {
    #[account(mut)] pub investor: Signer<'info>,
    #[account(mut,seeds=[b"round-v2",round.creator.as_ref(),&round.round_id.to_le_bytes()],bump=round.bump)]
    pub round: Account<'info,Round>,
    #[account(init_if_needed,payer=investor,space=8+RoundPosition::INIT_SPACE,seeds=[b"position-v2",round.key().as_ref(),investor.key().as_ref()],bump)]
    pub position: Account<'info,RoundPosition>,
    #[account(mut,address=round.vault,token::mint=mint,token::authority=round)] pub vault: Account<'info,TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=investor)] pub investor_token: Account<'info,TokenAccount>,
    #[account(address=round.mint)] pub mint: Account<'info,Mint>,
    pub token_program: Program<'info,Token>,
    pub system_program: Program<'info,System>,
}
pub fn invest(ctx:Context<InvestRound>,amount:u64) -> Result<()> {
    let r=&mut ctx.accounts.round;
    require!(r.status==1 && Clock::get()?.unix_timestamp<r.deadline,V2Error::InvalidState);
    let raised=add(r.raised,amount)?;
    require!(amount>0 && raised<=r.cap,V2Error::InvalidAmount);
    token::transfer_checked(CpiContext::new(ctx.accounts.token_program.key(),TransferChecked {
        from:ctx.accounts.investor_token.to_account_info(),mint:ctx.accounts.mint.to_account_info(),
        to:ctx.accounts.vault.to_account_info(),authority:ctx.accounts.investor.to_account_info()}),amount,ctx.accounts.mint.decimals)?;
    let p=&mut ctx.accounts.position;
    p.round=r.key(); p.investor=ctx.accounts.investor.key(); p.amount=add(p.amount,amount)?; p.bump=ctx.bumps.position;
    r.raised=raised; Ok(())
}
#[derive(Accounts)]
pub struct WithdrawRound<'info> {
    pub investor: Signer<'info>,
    #[account(mut,seeds=[b"round-v2",round.creator.as_ref(),&round.round_id.to_le_bytes()],bump=round.bump)]
    pub round: Account<'info,Round>,
    #[account(mut,seeds=[b"position-v2",round.key().as_ref(),investor.key().as_ref()],bump=position.bump,has_one=investor,has_one=round)]
    pub position: Account<'info,RoundPosition>,
    #[account(mut,address=round.vault,token::mint=mint,token::authority=round)] pub vault: Account<'info,TokenAccount>,
    #[account(mut,token::mint=mint,token::authority=investor)] pub investor_token: Account<'info,TokenAccount>,
    #[account(address=round.mint)] pub mint: Account<'info,Mint>,
    pub token_program: Program<'info,Token>,
}
pub fn withdraw(ctx:Context<WithdrawRound>,amount:u64,cancel:bool) -> Result<()> {
    let r=&mut ctx.accounts.round;
    if cancel { require!(cancellation_allowed(r.status,Clock::get()?.unix_timestamp,r.deadline),V2Error::CancellationLocked); }
    else { require!(r.status==3,V2Error::InvalidState); }
    let p=&mut ctx.accounts.position;
    require!(amount>0 && amount<=p.amount,V2Error::InvalidAmount);
    let id=r.round_id.to_le_bytes(); let bump=[r.bump];
    let seeds:&[&[u8]]=&[b"round-v2",r.creator.as_ref(),&id,&bump];
    token::transfer_checked(CpiContext::new_with_signer(ctx.accounts.token_program.key(),TransferChecked {
        from:ctx.accounts.vault.to_account_info(),mint:ctx.accounts.mint.to_account_info(),
        to:ctx.accounts.investor_token.to_account_info(),authority:r.to_account_info()},&[seeds]),amount,ctx.accounts.mint.decimals)?;
    p.amount=p.amount.checked_sub(amount).ok_or(V2Error::Overflow)?;
    if cancel { r.raised=r.raised.checked_sub(amount).ok_or(V2Error::Overflow)?; } Ok(())
}
#[derive(Accounts)]
pub struct ReviewMilestone<'info> {
    pub signer: Signer<'info>,
    #[account(seeds=[b"round-v2",round.creator.as_ref(),&round.round_id.to_le_bytes()],bump=round.bump)] pub round: Account<'info,Round>,
    #[account(mut,seeds=[b"milestone-v2",round.key().as_ref(),&milestone.index.to_le_bytes()],bump=milestone.bump,has_one=round)]
    pub milestone: Account<'info,Milestone>,
}
pub fn submit(ctx:Context<ReviewMilestone>,evidence_hash:[u8;32]) -> Result<()> {
    require!(ctx.accounts.signer.key()==ctx.accounts.round.creator,V2Error::Unauthorized);
    require!(ctx.accounts.round.status==2 && ctx.accounts.milestone.status<=1,V2Error::InvalidState);
    hash_present(&evidence_hash)?;
    ctx.accounts.milestone.evidence_hash=evidence_hash; ctx.accounts.milestone.status=1; Ok(())
}
pub fn approve(ctx:Context<ReviewMilestone>,review_hash:[u8;32]) -> Result<()> {
    require!(ctx.accounts.signer.key()==ctx.accounts.round.platform,V2Error::Unauthorized);
    require!(ctx.accounts.round.status==2 && ctx.accounts.milestone.status==1,V2Error::InvalidState);
    hash_present(&review_hash)?;
    ctx.accounts.milestone.review_hash=review_hash; ctx.accounts.milestone.status=2; Ok(())
}
#[derive(Accounts)]
pub struct ReleaseMilestone<'info> {
    #[account(mut,seeds=[b"round-v2",round.creator.as_ref(),&round.round_id.to_le_bytes()],bump=round.bump)] pub round: Account<'info,Round>,
    #[account(mut,seeds=[b"milestone-v2",round.key().as_ref(),&milestone.index.to_le_bytes()],bump=milestone.bump,has_one=round)]
    pub milestone: Account<'info,Milestone>,
    #[account(mut,address=round.vault,token::mint=mint,token::authority=round)] pub vault: Account<'info,TokenAccount>,
    #[account(mut,token::mint=mint,constraint=destination.owner==round.recipient @ V2Error::Unauthorized)] pub destination: Account<'info,TokenAccount>,
    #[account(address=round.mint)] pub mint: Account<'info,Mint>,
    pub token_program: Program<'info,Token>,
}
pub fn release(ctx:Context<ReleaseMilestone>) -> Result<()> {
    let r=&mut ctx.accounts.round; let m=&mut ctx.accounts.milestone;
    require!(r.status==2 && m.status==2,V2Error::InvalidState);
    require!(m.index==r.next_release,V2Error::InvalidMilestone);
    let released=add(r.released,m.amount)?;
    require!(released<=r.raised && released<=r.allocated,V2Error::InvalidAmount);
    let id=r.round_id.to_le_bytes();let bump=[r.bump];
    let seeds:&[&[u8]]=&[b"round-v2",r.creator.as_ref(),&id,&bump];
    token::transfer_checked(CpiContext::new_with_signer(ctx.accounts.token_program.key(),TransferChecked {
        from:ctx.accounts.vault.to_account_info(),mint:ctx.accounts.mint.to_account_info(),
        to:ctx.accounts.destination.to_account_info(),authority:r.to_account_info()},&[seeds]),m.amount,ctx.accounts.mint.decimals)?;
    r.released=released; r.next_release=r.next_release.checked_add(1).ok_or(V2Error::Overflow)?; m.status=3; Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cancellation_boundary() {
        let deadline=1_000_000;
        assert!(cancellation_allowed(1,deadline-CANCEL_WINDOW-1,deadline));
        assert!(!cancellation_allowed(1,deadline-CANCEL_WINDOW,deadline));
        assert!(!cancellation_allowed(1,deadline,deadline));
        assert!(!cancellation_allowed(2,0,deadline));
    }
    #[test] fn minimum_decides_success_not_cap() {
        assert_eq!(closed_status(199,200),3); assert_eq!(closed_status(200,200),2); assert_eq!(closed_status(600,200),2);
    }
    #[test] fn overflow_is_rejected() { assert!(add(u64::MAX,1).is_err()); }
}

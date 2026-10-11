use {
    anchor_lang::{
        AccountDeserialize,
        AccountSerialize,
        InstructionData,
        ToAccountMetas,
        
    },
    anchor_spl::token::ID as TOKEN_PROGRAM_ID,
    anchor_spl::token::TokenAccount,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    spl_token_interface::instruction as token_instruction,
    solana_transaction::Transaction,
    solana_system_interface::instruction as system_instruction,
    solana_instruction::Instruction,
};

fn create_mint(svm: &mut LiteSVM, payer: &Keypair) -> Keypair {
    let mint = Keypair::new();

    // Tamanho de uma conta Mint SPL Token.
    let mint_space = 82u64;

    let mint_rent = svm
        .minimum_balance_for_rent_exemption(mint_space as usize);

    let create_mint_account = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        mint_space,
        &TOKEN_PROGRAM_ID,
    );

    let initialize_mint = token_instruction::initialize_mint2(
        &TOKEN_PROGRAM_ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        6,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_mint_account, initialize_mint],
        Some(&payer.pubkey()),
        &[payer, &mint],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .expect("falha ao criar e inicializar o mint");

    mint
}

fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Keypair {
    let token_account = Keypair::new();

    let account_space = 165u64;

    let account_rent = svm
        .minimum_balance_for_rent_exemption(account_space as usize);

    let create_account = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        account_rent,
        account_space,
        &TOKEN_PROGRAM_ID,
    );

    let initialize_account = token_instruction::initialize_account3(
        &TOKEN_PROGRAM_ID,
        &token_account.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_account, initialize_account],
        Some(&payer.pubkey()),
        &[payer, &token_account],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .expect("falha ao criar token account");

    token_account
}

    const RENT_SYSVAR_ID: Pubkey =
        solana_pubkey::Pubkey::from_str_const(
        "SysvarRent111111111111111111111111111111111",
    );




use solana_account::Account;
use solana_clock3::Clock;
const START:i64=1_000_000;
const DEADLINE:i64=START+259_200;
fn pda(seeds:&[&[u8]])->Pubkey { Pubkey::find_program_address(seeds,&raftt::ID).0 }
fn ix<A:ToAccountMetas,D:InstructionData>(accounts:A,data:D)->Instruction {
    Instruction {program_id:raftt::ID,accounts:accounts.to_account_metas(None),data:data.data()}
}
struct RoundCtx {
    svm:LiteSVM,creator:Keypair,reviewer:Keypair,investor:Keypair,
    mint:Keypair,investor_token:Keypair,destination:Keypair,
    round:Pubkey,vault:Pubkey,position:Pubkey,config:Pubkey,
}
impl RoundCtx {
    fn tx(&mut self,ix:Instruction,role:u8)->Result<(),String> {
        self.svm.expire_blockhash();
        let mut signers=vec![&self.creator];
        if role==1 {signers.push(&self.reviewer);} if role==2 {signers.push(&self.investor);}
        let tx=Transaction::new_signed_with_payer(&[ix],Some(&self.creator.pubkey()),&signers,self.svm.latest_blockhash());
        self.svm.send_transaction(tx).map(|_|()).map_err(|e|e.meta.logs.join("\n"))
    }
    fn time(&mut self,now:i64){let mut clock:Clock=self.svm.get_sysvar();clock.unix_timestamp=now;self.svm.set_sysvar(&clock);}
    fn new()->Self {
        let mut svm=LiteSVM::new();
        svm.add_program_from_file(raftt::ID,concat!(env!("CARGO_MANIFEST_DIR"),"/../../target/deploy/raftt.so")).unwrap();
        let creator=Keypair::new();let reviewer=Keypair::new();let investor=Keypair::new();
        for k in [&creator,&reviewer,&investor] {svm.airdrop(&k.pubkey(),10_000_000_000).unwrap();}
        let mint=create_mint(&mut svm,&creator);
        let investor_token=create_token_account(&mut svm,&creator,&mint.pubkey(),&investor.pubkey());
        let destination=create_token_account(&mut svm,&creator,&mint.pubkey(),&creator.pubkey());
        // Fixture bypasses bootstrap only. SetupPlatform must separately be checked against a deployed upgradeable program.
        let (config,bump)=Pubkey::find_program_address(&[b"platform-v2"],&raftt::ID);
        let state=raftt::PlatformConfig {authority:reviewer.pubkey(),bump};
        let mut data=Vec::new();state.try_serialize(&mut data).unwrap();
        svm.set_account(config,Account {lamports:svm.minimum_balance_for_rent_exemption(data.len()),data,owner:raftt::ID,executable:false,rent_epoch:0}).unwrap();
        let round=pda(&[b"round-v2",creator.pubkey().as_ref(),&1u64.to_le_bytes()]);
        let vault=pda(&[b"vault-v2",round.as_ref()]);
        let position=pda(&[b"position-v2",round.as_ref(),investor.pubkey().as_ref()]);
        let mut ctx=Self {svm,creator,reviewer,investor,mint,investor_token,destination,round,vault,position,config};
        ctx.time(START);
        ctx.tx(ix(raftt::accounts::CreateRound {creator:ctx.creator.pubkey(),config,mint:ctx.mint.pubkey(),round,vault,system_program:anchor_lang::system_program::ID,token_program:TOKEN_PROGRAM_ID,rent:RENT_SYSVAR_ID},raftt::instruction::CreateRound {round_id:1,minimum:10_000_000,cap:20_000_000,deadline:DEADLINE,recipient:ctx.creator.pubkey(),plan_hash:[1;32]}),0).unwrap();
        let mint_to=token_instruction::mint_to(&TOKEN_PROGRAM_ID,&ctx.mint.pubkey(),&ctx.investor_token.pubkey(),&ctx.creator.pubkey(),&[],100_000_000).unwrap();ctx.tx(mint_to,0).unwrap();
        ctx
    }
    fn milestone(&self,index:u16)->Pubkey {pda(&[b"milestone-v2",self.round.as_ref(),&index.to_le_bytes()])}
    fn add(&mut self,index:u16,amount:u64)->Result<(),String> {
        self.tx(ix(raftt::accounts::AddMilestone {creator:self.creator.pubkey(),round:self.round,milestone:self.milestone(index),system_program:anchor_lang::system_program::ID},raftt::instruction::AddMilestone {index,amount,plan_hash:[2;32]}),0)
    }
    fn open(&mut self,role:u8)->Result<(),String> {
        let reviewer=if role==1 {self.reviewer.pubkey()} else {self.creator.pubkey()};
        self.tx(ix(raftt::accounts::ReviewRound {reviewer,round:self.round},raftt::instruction::OpenRound {review_hash:[3;32]}),role)
    }
    fn invest(&mut self,amount:u64)->Result<(),String>{
        self.tx(ix(raftt::accounts::InvestRound {investor:self.investor.pubkey(),round:self.round,position:self.position,vault:self.vault,investor_token:self.investor_token.pubkey(),mint:self.mint.pubkey(),token_program:TOKEN_PROGRAM_ID,system_program:anchor_lang::system_program::ID},raftt::instruction::InvestRound {amount}),2)
    }
    fn withdraw_accounts(&self)->raftt::accounts::WithdrawRound {
        raftt::accounts::WithdrawRound {investor:self.investor.pubkey(),round:self.round,position:self.position,vault:self.vault,investor_token:self.investor_token.pubkey(),mint:self.mint.pubkey(),token_program:TOKEN_PROGRAM_ID}
    }
    fn cancel(&mut self,amount:u64)->Result<(),String>{self.tx(ix(self.withdraw_accounts(),raftt::instruction::CancelRound {amount}),2)}
    fn refund(&mut self)->Result<(),String>{self.tx(ix(self.withdraw_accounts(),raftt::instruction::RefundRound {}),2)}
    fn close(&mut self)->Result<(),String>{self.tx(ix(raftt::accounts::CloseRound {round:self.round},raftt::instruction::CloseRound {}),0)}
    fn submit(&mut self,index:u16)->Result<(),String>{self.tx(ix(raftt::accounts::ReviewMilestone {signer:self.creator.pubkey(),round:self.round,milestone:self.milestone(index)},raftt::instruction::SubmitMilestone {evidence_hash:[4;32]}),0)}
    fn approve(&mut self,index:u16,role:u8)->Result<(),String>{let signer=if role==1 {self.reviewer.pubkey()} else {self.creator.pubkey()};self.tx(ix(raftt::accounts::ReviewMilestone {signer,round:self.round,milestone:self.milestone(index)},raftt::instruction::ApproveMilestone {review_hash:[5;32]}),role)}
    fn release(&mut self,index:u16,destination:Pubkey)->Result<(),String>{self.tx(ix(raftt::accounts::ReleaseMilestone {round:self.round,milestone:self.milestone(index),vault:self.vault,destination,mint:self.mint.pubkey(),token_program:TOKEN_PROGRAM_ID},raftt::instruction::ReleaseMilestone {}),0)}
    fn state(&self)->raftt::Round {let a=self.svm.get_account(&self.round).unwrap();raftt::Round::try_deserialize(&mut a.data.as_slice()).unwrap()}
    fn tokens(&self,address:Pubkey)->u64 {let a=self.svm.get_account(&address).unwrap();u64::from_le_bytes(a.data[64..72].try_into().unwrap())}
    fn planned()->Self {let mut s=Self::new();s.add(0,4_000_000).unwrap();s.add(1,6_000_000).unwrap();s.open(1).unwrap();s}
}
fn rejected(result:Result<(),String>,message:&str){let logs=result.unwrap_err();assert!(logs.contains(message),"Expected {message}: {logs}");}
#[test] fn review_and_budget_permissions(){
    let mut s=RoundCtx::new();rejected(s.open(0),"Unauthorized");rejected(s.open(1),"InvalidAmount");
    rejected(s.add(0,11_000_000),"InvalidAmount");s.add(0,10_000_000).unwrap();s.open(1).unwrap();
    rejected(s.add(1,1),"InvalidState");
}
#[test] fn cap_does_not_close_and_cancel_window_is_exact(){
    let mut s=RoundCtx::planned();s.invest(20_000_000).unwrap();assert_eq!(s.state().status,1);
    rejected(s.invest(1),"InvalidAmount");s.time(DEADLINE-raftt::CANCEL_WINDOW-1);s.cancel(5_000_000).unwrap();assert_eq!(s.state().raised,15_000_000);
    s.time(DEADLINE-raftt::CANCEL_WINDOW);rejected(s.cancel(1),"CancellationLocked");rejected(s.close(),"InvalidDeadline");
    s.time(DEADLINE);rejected(s.invest(1),"InvalidState");s.close().unwrap();assert_eq!(s.state().status,2);
}
#[test] fn failure_refund_and_duplicate_refund(){
    let mut s=RoundCtx::planned();s.invest(5_000_000).unwrap();rejected(s.refund(),"InvalidState");s.time(DEADLINE);s.close().unwrap();assert_eq!(s.state().status,3);
    s.refund().unwrap();assert_eq!(s.tokens(s.vault),0);assert_eq!(s.tokens(s.investor_token.pubkey()),100_000_000);assert_eq!(s.state().raised,5_000_000);
    rejected(s.refund(),"InvalidAmount");
}
#[test] fn installments_require_approval_order_destination_and_cannot_repeat(){
    let mut s=RoundCtx::planned();s.invest(10_000_000).unwrap();rejected(s.submit(0),"InvalidState");s.time(DEADLINE);s.close().unwrap();
    rejected(s.release(0,s.destination.pubkey()),"InvalidState");s.submit(0).unwrap();rejected(s.approve(0,0),"Unauthorized");s.approve(0,1).unwrap();
    s.submit(1).unwrap();s.approve(1,1).unwrap();rejected(s.release(1,s.destination.pubkey()),"InvalidMilestone");
    rejected(s.release(0,s.investor_token.pubkey()),"Unauthorized");s.release(0,s.destination.pubkey()).unwrap();assert_eq!(s.tokens(s.destination.pubkey()),4_000_000);assert_eq!(s.tokens(s.vault),6_000_000);
    rejected(s.release(0,s.destination.pubkey()),"InvalidState");s.release(1,s.destination.pubkey()).unwrap();assert_eq!(s.tokens(s.vault),0);assert_eq!(s.state().released,10_000_000);
}

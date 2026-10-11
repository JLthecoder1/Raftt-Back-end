use {
    anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas},
    anchor_spl::token::TokenAccount,
    anchor_spl::token::ID as TOKEN_PROGRAM_ID,
    litesvm::LiteSVM,
    solana_instruction::Instruction,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_system_interface::instruction as system_instruction,
    solana_transaction::Transaction,
    spl_token_interface::instruction as token_instruction,
};

const RENT_SYSVAR_ID: Pubkey =
    Pubkey::from_str_const("SysvarRent111111111111111111111111111111111");
const OFFERING_ID: u64 = 1;

fn create_mint(svm: &mut LiteSVM, payer: &Keypair) -> Keypair {
    let mint = Keypair::new();
    let mint_space = 82u64;

    let mint_rent = svm.minimum_balance_for_rent_exemption(mint_space as usize);

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

    let account_rent = svm.minimum_balance_for_rent_exemption(account_space as usize);

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

struct TestContext {
    svm: LiteSVM,
    payer: Keypair,
    mint: Keypair,
    investor_token_account: Keypair,
    offering_id: u64,
    offering: Pubkey,
    vault: Pubkey,
    position: Pubkey,
}

impl TestContext {
    fn new(target_amount: u64, initial_tokens: u64) -> Self {
        let mut svm = LiteSVM::new();

        svm.add_program_from_file(
            raftt::ID,
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/raftt.so"),
        )
        .unwrap();

        let payer = Keypair::new();

        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

        let mint = create_mint(&mut svm, &payer);

        let investor_token_account =
            create_token_account(&mut svm, &payer, &mint.pubkey(), &payer.pubkey());

        if initial_tokens > 0 {
            let mint_to_ix = token_instruction::mint_to(
                &TOKEN_PROGRAM_ID,
                &mint.pubkey(),
                &investor_token_account.pubkey(),
                &payer.pubkey(),
                &[],
                initial_tokens,
            )
            .unwrap();

            let tx = Transaction::new_signed_with_payer(
                &[mint_to_ix],
                Some(&payer.pubkey()),
                &[&payer],
                svm.latest_blockhash(),
            );

            svm.send_transaction(tx)
                .expect("falha ao criar tokens do investidor");
        }

        let offering_id = OFFERING_ID;
        let (offering, _) = Pubkey::find_program_address(
            &[
                b"offering",
                payer.pubkey().as_ref(),
                &offering_id.to_le_bytes(),
            ],
            &raftt::ID,
        );

        let (vault, _) = Pubkey::find_program_address(&[b"vault", offering.as_ref()], &raftt::ID);

        let (position, _) = Pubkey::find_program_address(
            &[b"position", offering.as_ref(), payer.pubkey().as_ref()],
            &raftt::ID,
        );

        let mut ctx = Self {
            svm,
            payer,
            mint,
            investor_token_account,
            offering_id,
            offering,
            vault,
            position,
        };

        ctx.initialize(target_amount);
        ctx
    }

    fn initialize(&mut self, target_amount: u64) {
        let initialize_ix = Instruction {
            program_id: raftt::ID,
            accounts: raftt::accounts::Initialize {
                user: self.payer.pubkey(),
                mint: self.mint.pubkey(),
                offering: self.offering,
                vault: self.vault,
                system_program: anchor_lang::system_program::ID,
                token_program: TOKEN_PROGRAM_ID,
                rent: RENT_SYSVAR_ID,
            }
            .to_account_metas(None),
            data: raftt::instruction::Initialize {
                offering_id: self.offering_id,
                target_amount,
            }
            .data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[initialize_ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.svm.latest_blockhash(),
        );

        self.svm
            .send_transaction(tx)
            .expect("falha ao inicializar offering");
    }

    fn invest_ix(&self, amount: u64) -> Instruction {
        Instruction {
            program_id: raftt::ID,
            accounts: raftt::accounts::Invest {
                investor: self.payer.pubkey(),
                investor_token_account: self.investor_token_account.pubkey(),
                mint: self.mint.pubkey(),
                offering: self.offering,
                position: self.position,
                vault: self.vault,
                token_program: TOKEN_PROGRAM_ID,
                system_program: anchor_lang::system_program::ID,
            }
            .to_account_metas(None),
            data: raftt::instruction::Invest { amount }.data(),
        }
    }

    fn send_invest(
        &mut self,
        amount: u64,
    ) -> Result<(), litesvm::types::FailedTransactionMetadata> {
        let ix = self.invest_ix(amount);

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).map(|_| ())
    }

    fn send_invest_with_accounts(
        &mut self,
        investor_token_account: Pubkey,
        mint: Pubkey,
        vault: Pubkey,
        amount: u64,
    ) -> Result<(), litesvm::types::FailedTransactionMetadata> {
        let instruction = Instruction {
            program_id: raftt::ID,
            accounts: raftt::accounts::Invest {
                investor: self.payer.pubkey(),
                investor_token_account,
                mint,
                offering: self.offering,
                position: self.position,
                vault,
                token_program: TOKEN_PROGRAM_ID,
                system_program: anchor_lang::system_program::ID,
            }
            .to_account_metas(None),
            data: raftt::instruction::Invest { amount }.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).map(|_| ())
    }

    fn send_fail_offering(&mut self) -> Result<(), litesvm::types::FailedTransactionMetadata> {
        let instruction = Instruction {
            program_id: raftt::ID,
            accounts: raftt::accounts::FailOffering {
                offering: self.offering,
                authority: self.payer.pubkey(),
            }
            .to_account_metas(None),
            data: raftt::instruction::FailOffering {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).map(|_| ())
    }

    fn send_refund(&mut self) -> Result<(), litesvm::types::FailedTransactionMetadata> {
        let instruction = Instruction {
            program_id: raftt::ID,
            accounts: raftt::accounts::Refund {
                investor: self.payer.pubkey(),
                offering: self.offering,
                position: self.position,
                vault: self.vault,
                investor_token_account: self.investor_token_account.pubkey(),
                token_program: TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: raftt::instruction::Refund {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).map(|_| ())
    }

    fn send_release(&mut self) -> Result<(), litesvm::types::FailedTransactionMetadata> {
        let instruction = Instruction {
            program_id: raftt::ID,
            accounts: raftt::accounts::Release {
                authority: self.payer.pubkey(),
                offering: self.offering,
                vault: self.vault,
                destination: self.investor_token_account.pubkey(),
                token_program: TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: raftt::instruction::Release {}.data(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.svm.latest_blockhash(),
        );

        self.svm.send_transaction(tx).map(|_| ())
    }
}

#[test]
fn test_invest() {
    let mut svm = LiteSVM::new();

    svm.add_program_from_file(
        raftt::ID,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/raftt.so"),
    )
    .unwrap();

    let payer = Keypair::new();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = create_mint(&mut svm, &payer);

    let investor_token_account =
        create_token_account(&mut svm, &payer, &mint.pubkey(), &payer.pubkey());

    let mint_to_ix = token_instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &mint.pubkey(),
        &investor_token_account.pubkey(),
        &payer.pubkey(),
        &[],
        500_000,
    )
    .unwrap();

    let offering_id = OFFERING_ID;
    let (offering, _) = Pubkey::find_program_address(
        &[
            b"offering",
            payer.pubkey().as_ref(),
            &offering_id.to_le_bytes(),
        ],
        &raftt::ID,
    );

    let (vault, _) = Pubkey::find_program_address(&[b"vault", offering.as_ref()], &raftt::ID);

    let (position, _) = Pubkey::find_program_address(
        &[b"position", offering.as_ref(), payer.pubkey().as_ref()],
        &raftt::ID,
    );

    let initialize_ix = Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::Initialize {
            user: payer.pubkey(),
            mint: mint.pubkey(),
            offering,
            vault,
            system_program: anchor_lang::system_program::ID,
            token_program: TOKEN_PROGRAM_ID,
            rent: RENT_SYSVAR_ID,
        }
        .to_account_metas(None),
        data: raftt::instruction::Initialize {
            offering_id,
            target_amount: 500_000,
        }
        .data(),
    };

    let invest_ix = Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::Invest {
            investor: payer.pubkey(),
            investor_token_account: investor_token_account.pubkey(),
            mint: mint.pubkey(),
            offering,
            vault,
            position,
            token_program: TOKEN_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: raftt::instruction::Invest { amount: 500_000 }.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix, initialize_ix, invest_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .expect("falha ao executar mint, initialize e invest");

    let offering_account = svm.get_account(&offering).expect("offering não encontrada");

    let offering_state =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice())
            .expect("falha ao desserializar offering");

    assert_eq!(offering_state.raised_amount, 500_000);
    assert!(offering_state.is_closed);

    let investor_account = svm
        .get_account(&investor_token_account.pubkey())
        .expect("conta do investidor não encontrada");

    let investor_token = TokenAccount::try_deserialize(&mut investor_account.data.as_slice())
        .expect("falha ao desserializar conta do investidor");

    assert_eq!(investor_token.amount, 0);

    let vault_account = svm.get_account(&vault).expect("vault não encontrado");

    let vault_token = TokenAccount::try_deserialize(&mut vault_account.data.as_slice())
        .expect("falha ao desserializar vault");

    assert_eq!(vault_token.amount, 500_000);
}

#[test]
fn test_invest_zero_amount_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    let result = ctx.send_invest(0);

    assert!(result.is_err());
}

#[test]
fn test_invest_exceeds_target_fails() {
    let mut ctx = TestContext::new(500_000, 600_000);

    let result = ctx.send_invest(600_000);

    assert!(result.is_err());
}

#[test]
fn test_invest_after_target_reached_fails_without_changing_balances() {
    let mut ctx = TestContext::new(500_000, 500_000);
    ctx.send_invest(500_000).unwrap();

    let result = ctx.send_invest(1);

    assert!(result.is_err());

    let investor_account = ctx
        .svm
        .get_account(&ctx.investor_token_account.pubkey())
        .unwrap();
    let investor_token = TokenAccount::try_deserialize(&mut investor_account.data.as_slice())
        .expect("falha ao desserializar conta do investidor");
    assert_eq!(investor_token.amount, 0);

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.amount, 500_000);

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();
    let offering =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();
    assert_eq!(offering.raised_amount, 500_000);
}

#[test]
fn test_invest_rejects_incompatible_spl_accounts_without_moving_funds() {
    let mut ctx = TestContext::new(500_000, 500_000);
    let other_mint = create_mint(&mut ctx.svm, &ctx.payer);
    let other_owner = Keypair::new();
    let wrong_owner_token_account = create_token_account(
        &mut ctx.svm,
        &ctx.payer,
        &ctx.mint.pubkey(),
        &other_owner.pubkey(),
    );
    let wrong_vault = create_token_account(
        &mut ctx.svm,
        &ctx.payer,
        &ctx.mint.pubkey(),
        &ctx.payer.pubkey(),
    );

    assert!(
        ctx.send_invest_with_accounts(
            ctx.investor_token_account.pubkey(),
            other_mint.pubkey(),
            ctx.vault,
            100_000,
        )
        .is_err(),
        "deveria rejeitar mint diferente do configurado na oferta"
    );

    assert!(
        ctx.send_invest_with_accounts(
            wrong_owner_token_account.pubkey(),
            ctx.mint.pubkey(),
            ctx.vault,
            100_000,
        )
        .is_err(),
        "deveria rejeitar token account de outro proprietário"
    );

    assert!(
        ctx.send_invest_with_accounts(
            ctx.investor_token_account.pubkey(),
            ctx.mint.pubkey(),
            wrong_vault.pubkey(),
            100_000,
        )
        .is_err(),
        "deveria rejeitar cofre diferente do configurado na oferta"
    );

    let investor_account = ctx
        .svm
        .get_account(&ctx.investor_token_account.pubkey())
        .unwrap();
    let investor_token = TokenAccount::try_deserialize(&mut investor_account.data.as_slice())
        .expect("falha ao desserializar conta do investidor");
    assert_eq!(investor_token.amount, 500_000);

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.amount, 0);

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();
    let offering =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();
    assert_eq!(offering.raised_amount, 0);
}

#[test]
fn test_invest_partial_then_complete() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(300_000).unwrap();

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();

    let mut offering_state =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();

    assert_eq!(offering_state.raised_amount, 300_000);
    assert!(!offering_state.is_closed);

    ctx.send_invest(200_000).unwrap();

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();

    offering_state =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();

    assert_eq!(offering_state.raised_amount, 500_000);
    assert!(offering_state.is_closed);
}

#[test]
fn test_invest_twice_then_complete() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(300_000).unwrap();
    ctx.send_invest(200_000).unwrap();

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();

    let offering_state =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();

    assert_eq!(offering_state.raised_amount, 500_000);
    assert!(offering_state.is_closed);

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();

    let vault_token = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();

    assert_eq!(vault_token.amount, 500_000);

    let position_account = ctx
        .svm
        .get_account(&ctx.position)
        .expect("position não encontrada");

    let position_state =
        raftt::state::Position::try_deserialize(&mut position_account.data.as_slice())
            .expect("falha ao desserializar position");

    assert_eq!(position_state.offering, ctx.offering);
    assert_eq!(position_state.investor, ctx.payer.pubkey());
    assert_eq!(position_state.amount, 500_000);
}

#[test]
fn test_refund() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(300_000).unwrap();
    ctx.send_fail_offering().unwrap();
    ctx.send_refund().unwrap();

    let position_account = ctx
        .svm
        .get_account(&ctx.position)
        .expect("position não encontrada");

    let position_state =
        raftt::state::Position::try_deserialize(&mut position_account.data.as_slice())
            .expect("falha ao desserializar position");

    assert_eq!(position_state.amount, 0);

    let vault_account = ctx
        .svm
        .get_account(&ctx.vault)
        .expect("vault não encontrada");

    let vault_token = TokenAccount::try_deserialize(&mut vault_account.data.as_slice())
        .expect("falha ao ler vault");

    assert_eq!(vault_token.amount, 0);

    let investor_account = ctx
        .svm
        .get_account(&ctx.investor_token_account.pubkey())
        .expect("conta do investidor não encontrada");

    let investor_token = TokenAccount::try_deserialize(&mut investor_account.data.as_slice())
        .expect("falha ao ler conta do investidor");

    assert_eq!(investor_token.amount, 500_000);
}

#[test]
fn test_refund_only_returns_the_calling_investors_position() {
    let mut ctx = TestContext::new(500_000, 500_000);
    let second_investor = Keypair::new();

    ctx.svm
        .airdrop(&second_investor.pubkey(), 1_000_000_000)
        .unwrap();

    let second_investor_token_account = create_token_account(
        &mut ctx.svm,
        &ctx.payer,
        &ctx.mint.pubkey(),
        &second_investor.pubkey(),
    );

    let mint_to_ix = token_instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &ctx.mint.pubkey(),
        &second_investor_token_account.pubkey(),
        &ctx.payer.pubkey(),
        &[],
        300_000,
    )
    .unwrap();

    let mint_tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );

    ctx.svm
        .send_transaction(mint_tx)
        .expect("falha ao criar tokens para o segundo investidor");

    let second_position = Pubkey::find_program_address(
        &[
            b"position",
            ctx.offering.as_ref(),
            second_investor.pubkey().as_ref(),
        ],
        &raftt::ID,
    )
    .0;

    let second_invest_ix = Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::Invest {
            investor: second_investor.pubkey(),
            investor_token_account: second_investor_token_account.pubkey(),
            mint: ctx.mint.pubkey(),
            offering: ctx.offering,
            position: second_position,
            vault: ctx.vault,
            token_program: TOKEN_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: raftt::instruction::Invest { amount: 200_000 }.data(),
    };

    let second_invest_tx = Transaction::new_signed_with_payer(
        &[second_invest_ix],
        Some(&second_investor.pubkey()),
        &[&second_investor],
        ctx.svm.latest_blockhash(),
    );

    ctx.svm
        .send_transaction(second_invest_tx)
        .expect("falha no investimento do segundo investidor");
    ctx.send_invest(100_000).unwrap();
    ctx.send_fail_offering().unwrap();
    ctx.send_refund().unwrap();

    let first_position_account = ctx.svm.get_account(&ctx.position).unwrap();
    let first_position =
        raftt::state::Position::try_deserialize(&mut first_position_account.data.as_slice())
            .unwrap();
    assert_eq!(first_position.amount, 0);

    let second_position_account = ctx.svm.get_account(&second_position).unwrap();
    let second_position_state =
        raftt::state::Position::try_deserialize(&mut second_position_account.data.as_slice())
            .unwrap();
    assert_eq!(second_position_state.amount, 200_000);

    let first_token_account = ctx
        .svm
        .get_account(&ctx.investor_token_account.pubkey())
        .unwrap();
    let first_token =
        TokenAccount::try_deserialize(&mut first_token_account.data.as_slice()).unwrap();
    assert_eq!(first_token.amount, 500_000);

    let second_token_account = ctx
        .svm
        .get_account(&second_investor_token_account.pubkey())
        .unwrap();
    let second_token =
        TokenAccount::try_deserialize(&mut second_token_account.data.as_slice()).unwrap();
    assert_eq!(second_token.amount, 100_000);

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.amount, 200_000);

    let second_refund_ix = Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::Refund {
            investor: second_investor.pubkey(),
            offering: ctx.offering,
            position: second_position,
            vault: ctx.vault,
            investor_token_account: second_investor_token_account.pubkey(),
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: raftt::instruction::Refund {}.data(),
    };

    let second_refund_tx = Transaction::new_signed_with_payer(
        &[second_refund_ix],
        Some(&second_investor.pubkey()),
        &[&second_investor],
        ctx.svm.latest_blockhash(),
    );

    ctx.svm
        .send_transaction(second_refund_tx)
        .expect("falha no reembolso do segundo investidor");

    let second_token_account = ctx
        .svm
        .get_account(&second_investor_token_account.pubkey())
        .unwrap();
    let second_token =
        TokenAccount::try_deserialize(&mut second_token_account.data.as_slice()).unwrap();
    assert_eq!(second_token.amount, 300_000);

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.amount, 0);
}

#[test]
fn test_refund_twice_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(300_000).unwrap();
    ctx.send_fail_offering().unwrap();
    ctx.send_refund().unwrap();

    let result = ctx.send_refund();

    assert!(result.is_err());
}

#[test]
fn test_refund_without_investment_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_fail_offering().unwrap();

    let result = ctx.send_refund();

    assert!(result.is_err());
}

#[test]
fn test_refund_before_offering_fails_without_changing_balances() {
    let mut ctx = TestContext::new(500_000, 500_000);
    ctx.send_invest(300_000).unwrap();

    let result = ctx.send_refund();

    assert!(result.is_err());

    let investor_account = ctx
        .svm
        .get_account(&ctx.investor_token_account.pubkey())
        .unwrap();
    let investor_token = TokenAccount::try_deserialize(&mut investor_account.data.as_slice())
        .expect("falha ao desserializar conta do investidor");
    assert_eq!(investor_token.amount, 200_000);

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.amount, 300_000);

    let position_account = ctx.svm.get_account(&ctx.position).unwrap();
    let position =
        raftt::state::Position::try_deserialize(&mut position_account.data.as_slice()).unwrap();
    assert_eq!(position.amount, 300_000);
}

#[test]
fn test_invest_failed_offering_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_fail_offering().unwrap();

    let result = ctx.send_invest(100_000);

    assert!(result.is_err());
}

#[test]
fn test_release_after_target_reached() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(500_000).unwrap();
    ctx.send_release().unwrap();

    let destination_account = ctx
        .svm
        .get_account(&ctx.investor_token_account.pubkey())
        .expect("conta de destino não encontrada");

    let destination = TokenAccount::try_deserialize(&mut destination_account.data.as_slice())
        .expect("falha ao ler conta de destino");

    assert_eq!(destination.amount, 500_000);

    let vault_account = ctx
        .svm
        .get_account(&ctx.vault)
        .expect("vault não encontrado");

    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice())
        .expect("falha ao ler vault");

    assert_eq!(vault.amount, 0);

    let offering_account = ctx
        .svm
        .get_account(&ctx.offering)
        .expect("oferta não encontrada");

    let offering = raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice())
        .expect("falha ao ler oferta");

    assert!(offering.is_released);
}

#[test]
fn test_release_before_target_reached_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(300_000).unwrap();

    let result = ctx.send_release();

    assert!(result.is_err());

    let vault_account = ctx
        .svm
        .get_account(&ctx.vault)
        .expect("vault não encontrado");

    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice())
        .expect("falha ao ler vault");

    assert_eq!(vault.amount, 300_000);
}

#[test]
fn test_release_failed_offering_fails_without_moving_funds() {
    let mut ctx = TestContext::new(500_000, 500_000);
    ctx.send_invest(300_000).unwrap();
    ctx.send_fail_offering().unwrap();

    let result = ctx.send_release();

    assert!(result.is_err());

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.amount, 300_000);

    let destination_account = ctx
        .svm
        .get_account(&ctx.investor_token_account.pubkey())
        .unwrap();
    let destination =
        TokenAccount::try_deserialize(&mut destination_account.data.as_slice()).unwrap();
    assert_eq!(destination.amount, 200_000);

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();
    let offering =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();
    assert!(offering.is_failed);
    assert!(!offering.is_released);
}

#[test]
fn test_release_wrong_authority_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(500_000).unwrap();

    let attacker = Keypair::new();

    ctx.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let instruction = Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::Release {
            authority: attacker.pubkey(),
            offering: ctx.offering,
            vault: ctx.vault,
            destination: ctx.investor_token_account.pubkey(),
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: raftt::instruction::Release {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&attacker.pubkey()),
        &[&attacker],
        ctx.svm.latest_blockhash(),
    );

    let result = ctx.svm.send_transaction(tx);

    assert!(result.is_err());

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();

    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();

    assert_eq!(vault.amount, 500_000);
}

#[test]
fn test_release_twice_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(500_000).unwrap();
    ctx.send_release().unwrap();

    let result = ctx.send_release();

    assert!(result.is_err());

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();

    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();

    assert_eq!(vault.amount, 0);
}

#[test]
fn test_fail_offering_wrong_authority_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    let attacker = Keypair::new();

    ctx.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let instruction = Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::FailOffering {
            offering: ctx.offering,
            authority: attacker.pubkey(),
        }
        .to_account_metas(None),
        data: raftt::instruction::FailOffering {}.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&attacker.pubkey()),
        &[&attacker],
        ctx.svm.latest_blockhash(),
    );

    let result = ctx.svm.send_transaction(tx);

    assert!(result.is_err());

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();

    let offering =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();

    assert!(!offering.is_failed);
}

#[test]
fn test_fail_offering_after_target_reached_fails() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_invest(500_000).unwrap();

    let result = ctx.send_fail_offering();

    assert!(result.is_err());

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();

    let offering =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();

    assert!(offering.is_closed);
    assert!(!offering.is_failed);
}

#[test]
fn test_fail_offering_closes_offering() {
    let mut ctx = TestContext::new(500_000, 500_000);

    ctx.send_fail_offering().unwrap();

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();

    let offering =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();

    assert!(offering.is_failed);
    assert!(offering.is_closed);
}

#[test]
fn test_fail_offering_twice_fails_without_changing_state() {
    let mut ctx = TestContext::new(500_000, 500_000);
    ctx.send_invest(300_000).unwrap();
    ctx.send_fail_offering().unwrap();

    let result = ctx.send_fail_offering();

    assert!(result.is_err());

    let offering_account = ctx.svm.get_account(&ctx.offering).unwrap();
    let offering =
        raftt::state::Offering::try_deserialize(&mut offering_account.data.as_slice()).unwrap();
    assert!(offering.is_closed);
    assert!(offering.is_failed);
    assert_eq!(offering.raised_amount, 300_000);

    let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
    let vault = TokenAccount::try_deserialize(&mut vault_account.data.as_slice()).unwrap();
    assert_eq!(vault.amount, 300_000);
}

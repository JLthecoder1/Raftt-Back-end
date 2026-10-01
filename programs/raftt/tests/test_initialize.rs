
use {
    anchor_lang::{
        system_program,
        AccountDeserialize,
        InstructionData,
        ToAccountMetas,
    },
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
    solana_pubkey::Pubkey::from_str_const(
        "SysvarRent111111111111111111111111111111111",
    );

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();

    svm.add_program_from_file(
        raftt::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/raftt.so"
        ),
    )
    .unwrap();

    let payer = Keypair::new();

    svm.airdrop(&payer.pubkey(), 1_000_000_000)
        .unwrap();

    (svm, payer)
}

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

fn offering_pda(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"offering", user.as_ref()],
        &raftt::ID,
    )
}

fn vault_pda(offering: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"vault", offering.as_ref()],
        &raftt::ID,
    )
}

fn initialize_ix(
    user: Pubkey,
    offering: Pubkey,
    vault: Pubkey,
    mint: Pubkey,
    target_amount: u64,
) -> Instruction {
    Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::Initialize {
            user,
            mint,
            offering,
            vault,
            system_program: system_program::ID,
            token_program: TOKEN_PROGRAM_ID,
            rent: RENT_SYSVAR_ID,
        }
        .to_account_metas(None),
        data: raftt::instruction::Initialize {
            target_amount,
        }
        .data(),
    }
}

fn tx_for(
    svm: &LiteSVM,
    payer: &Keypair,
    ix: Instruction,
) -> Transaction {
    Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        svm.latest_blockhash(),
    )
}

#[test]
fn test_initialize() {
    let (mut svm, payer) = setup();

    let target_amount: u64 = 2_000_000;

    let (offering, bump) = offering_pda(&payer.pubkey());
    let (vault, _vault_bump) = vault_pda(&offering);

    // Cria um Mint SPL Token real.
    let mint = create_mint(&mut svm, &payer);

    let ix = initialize_ix(
        payer.pubkey(),
        offering,
        vault,
        mint.pubkey(),
        target_amount,
    );

    let tx = tx_for(&svm, &payer, ix);

    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("transação falhou: {e:#?}"));

    let account = svm
        .get_account(&offering)
        .expect("conta offering não foi criada");

    assert_eq!(account.owner, raftt::ID);

    let state =
        raftt::state::Offering::try_deserialize(
            &mut account.data.as_slice(),
        )
        .unwrap();

    assert_eq!(state.authority, payer.pubkey());
    assert_eq!(state.mint, mint.pubkey());
    assert_eq!(state.target_amount, target_amount);
    assert_eq!(state.raised_amount, 0);
    assert_eq!(state.is_closed, false);
    assert_eq!(state.vault, vault);
    assert_eq!(state.bump, bump);
}

#[test]
fn test_initialize_twice_fails() {
    let (mut svm, payer) = setup();

    let (offering, _) = offering_pda(&payer.pubkey());
    let (vault, _) = vault_pda(&offering);

    let mint = create_mint(&mut svm, &payer);

    let tx = tx_for(
        &svm,
        &payer,
        initialize_ix(
            payer.pubkey(),
            offering,
            vault,
            mint.pubkey(),
            2_000_000,
        ),
    );

    svm.send_transaction(tx).unwrap();

    let tx = tx_for(
        &svm,
        &payer,
        initialize_ix(
            payer.pubkey(),
            offering,
            vault,
            mint.pubkey(),
            3_000_000,
        ),
    );

    let err = svm.send_transaction(tx).unwrap_err();

    let logs = err.meta.logs.join("\n");

    assert!(
        logs.contains("already in use"),
        "falhou por outro motivo:\n{logs}"
    );

    let account = svm.get_account(&offering).unwrap();

    let state =
        raftt::state::Offering::try_deserialize(
            &mut account.data.as_slice(),
        )
        .unwrap();

    assert_eq!(state.target_amount, 2_000_000);
}

#[test]
fn test_initialize_wrong_pda_fails() {
    let (mut svm, payer) = setup();

    let fake_offering = Pubkey::new_unique();
    let (vault, _) = vault_pda(&fake_offering);

    let mint = create_mint(&mut svm, &payer);

    let tx = tx_for(
        &svm,
        &payer,
        initialize_ix(
            payer.pubkey(),
            fake_offering,
            vault,
            mint.pubkey(),
            2_000_000,
        ),
    );

    let err = svm.send_transaction(tx).unwrap_err();

    let logs = err.meta.logs.join("\n");

    assert!(
        logs.contains("ConstraintSeeds"),
        "falhou por outro motivo:\n{logs}"
    );

    assert!(svm.get_account(&fake_offering).is_none());
}

#[test]
fn test_initialize_zero_target_fails() {
    let (mut svm, payer) = setup();

    let (offering, _) = offering_pda(&payer.pubkey());
    let (vault, _) = vault_pda(&offering);

    let mint = create_mint(&mut svm, &payer);

    let tx = tx_for(
        &svm,
        &payer,
        initialize_ix(
            payer.pubkey(),
            offering,
            vault,
            mint.pubkey(),
            0,
        ),
    );

    let err = svm.send_transaction(tx).unwrap_err();

    let logs = err.meta.logs.join("\n");

    assert!(
        logs.contains("InvalidTargetAmount"),
        "falhou por outro motivo:\n{logs}"
    );

    assert!(svm.get_account(&offering).is_none());
}
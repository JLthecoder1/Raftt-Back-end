use {
    anchor_lang::{system_program, AccountDeserialize, InstructionData, ToAccountMetas},
    litesvm::LiteSVM,
    solana_instruction::Instruction,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(
        raftt::ID,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/raftt.so"),
    )
    .unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    (svm, payer)
}

fn offering_pda(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"offering", user.as_ref()], &raftt::ID)
}

fn initialize_ix(user: Pubkey, offering: Pubkey, target_amount: u64) -> Instruction {
    Instruction {
        program_id: raftt::ID,
        accounts: raftt::accounts::Initialize {
            user,
            offering,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: raftt::instruction::Initialize { target_amount }.data(),
    }
}

fn tx_for(svm: &LiteSVM, payer: &Keypair, ix: Instruction) -> Transaction {
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

    let tx = tx_for(&svm, &payer, initialize_ix(payer.pubkey(), offering, target_amount));
    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("transação falhou: {e:#?}"));

    let account = svm
        .get_account(&offering)
        .expect("conta offering não foi criada");
    assert_eq!(account.owner, raftt::ID);
    let state = raftt::state::Offering::try_deserialize(&mut account.data.as_slice()).unwrap();

    assert_eq!(state.authority, payer.pubkey());
    assert_eq!(state.target_amount, target_amount);
    assert_eq!(state.raised_amount, 0);
    assert_eq!(state.bump, bump);
}

#[test]
fn test_initialize_twice_fails() {
    let (mut svm, payer) = setup();
    let (offering, _) = offering_pda(&payer.pubkey());

    let tx = tx_for(&svm, &payer, initialize_ix(payer.pubkey(), offering, 2_000_000));
    svm.send_transaction(tx).unwrap();

    // target_amount diferente para a transação não ser tratada como duplicada
    let tx = tx_for(&svm, &payer, initialize_ix(payer.pubkey(), offering, 3_000_000));
    let err = svm.send_transaction(tx).unwrap_err();
    let logs = err.meta.logs.join("\n");
    assert!(logs.contains("already in use"), "falhou por outro motivo:\n{logs}");

    // o estado original não pode ter sido sobrescrito
    let account = svm.get_account(&offering).unwrap();
    let state = raftt::state::Offering::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(state.target_amount, 2_000_000);
}

#[test]
fn test_initialize_wrong_pda_fails() {
    let (mut svm, payer) = setup();
    let fake_offering = Pubkey::new_unique();

    let tx = tx_for(&svm, &payer, initialize_ix(payer.pubkey(), fake_offering, 2_000_000));

    let err = svm.send_transaction(tx).unwrap_err();
    let logs = err.meta.logs.join("\n");
    assert!(logs.contains("ConstraintSeeds"), "falhou por outro motivo:\n{logs}");

    assert!(svm.get_account(&fake_offering).is_none());
}
#[test]
fn test_initialize_zero_target_fails() {
    let (mut svm, payer) = setup();
    let (offering, _) = offering_pda(&payer.pubkey());

    let tx = tx_for(&svm, &payer, initialize_ix(payer.pubkey(), offering, 0));

    let err = svm.send_transaction(tx).unwrap_err();
    let logs = err.meta.logs.join("\n");
    assert!(logs.contains("InvalidTargetAmount"), "falhou por outro motivo:\n{logs}");

    // nenhuma conta pode ter sido criada
    assert!(svm.get_account(&offering).is_none());
}
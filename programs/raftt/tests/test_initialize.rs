
use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    solana_program_test::{processor, ProgramTest},
    solana_sdk::{
        account::Account,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    },
};

#[tokio::test]
async fn test_initialize() {
    let program_id = raftt::id();
    let payer = Keypair::new();
    let target_amount = 2_000_000;
    let (offering, bump) = Pubkey::find_program_address(
        &[b"offering", payer.pubkey().as_ref()],
        &program_id,
    );

    let mut program_test = ProgramTest::new("raftt", program_id, processor!(raftt::entrypoint));
    program_test.add_account(
        payer.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &raftt::instruction::Initialize { target_amount }.data(),
        raftt::accounts::Initialize {
            user: payer.pubkey(),
            offering,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    let offering_account = banks_client.get_account(offering).await.unwrap().unwrap();
    let mut data: &[u8] = &offering_account.data;
    let offering_state = raftt::state::Offering::try_deserialize(&mut data).unwrap();
    assert_eq!(offering_state.authority, payer.pubkey());
    assert_eq!(offering_state.target_amount, target_amount);
    assert_eq!(offering_state.raised_amount, 0);
    assert_eq!(offering_state.bump, bump);
}

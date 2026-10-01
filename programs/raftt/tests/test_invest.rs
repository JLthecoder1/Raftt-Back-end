use {
    anchor_lang::{
        AccountDeserialize,
        InstructionData,
        ToAccountMetas,
    },
    anchor_spl::token::ID as TOKEN_PROGRAM_ID,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    spl_token_interface::instruction as token_instruction,
    solana_transaction::Transaction,
    solana_system_interface::instruction as system_instruction,
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

#[test]
fn test_invest() {
    let mut svm = LiteSVM::new();

    let payer = Keypair::new();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    let mint = create_mint(&mut svm, &payer);

    let investor_token_account = create_token_account(
        &mut svm,
        &payer,
        &mint.pubkey(),
        &payer.pubkey(),
    );

    let mint_to_ix = token_instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        &mint.pubkey(),
        &investor_token_account.pubkey(),
        &payer.pubkey(),
        &[],
        500_000,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .expect("falha ao enviar tokens para o investidor");
}

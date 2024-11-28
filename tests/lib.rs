use anchor_lang::prelude::*;
use anchor_lang::system_program;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use solana_sdk::account::Account;
use env_logger;

use anchor_nft::instruction::mint_asset;
use anchor_nft::state::{CandyMachine, CandyMachineData, ConfigLine, MintAssetArgs};

#[tokio::test]
async fn test_mint_nft() {
    // Initialize the logger
    env_logger::init();

    let program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "anchor_nft",
        program_id,
        processor!(anchor_nft::entry),
    );

    let payer = Keypair::new();
    let authority_pda = Keypair::new();
    let asset_owner = Keypair::new();
    let asset = Keypair::new();
    let collection = Keypair::new();
    let candy_machine = Keypair::new();

    program_test.add_account(
        candy_machine.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![0; CandyMachine::LEN],
            owner: program_id,
            ..Account::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let candy_machine_data = CandyMachineData {
        items_available: 1,
    };

    let candy_machine_account = CandyMachine {
        items_redeemed: 0,
        data: candy_machine_data,
        collection_mint: collection.pubkey(),
    };

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::create_account(
            &payer.pubkey(),
            &candy_machine.pubkey(),
            1_000_000_000,
            CandyMachine::LEN as u64,
            &program_id,
        )],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &candy_machine], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let mint_args = MintAssetArgs {
        plugins: vec![],
    };

    let accounts = anchor_nft::accounts::MintAsset {
        authority_pda: authority_pda.pubkey(),
        payer: payer.pubkey(),
        asset_owner: asset_owner.pubkey(),
        asset: asset.pubkey(),
        collection: collection.pubkey(),
        mpl_core_program: system_program::ID,
        system_program: system_program::ID,
        sysvar_instructions: None,
        recent_slothashes: system_program::ID,
        candy_machine: candy_machine.pubkey(),
    };

    let ix = mint_asset(
        program_id,
        accounts,
        mint_args,
    );

    let mut transaction = Transaction::new_with_payer(
        &[ix],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[&payer, &authority_pda, &asset_owner, &asset, &collection, &candy_machine], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Add assertions to verify the NFT was minted correctly
    let candy_machine_account = banks_client
        .get_account(candy_machine.pubkey())
        .await
        .expect("get_account")
        .expect("candy_machine_account not found");

    let candy_machine_state: CandyMachine = CandyMachine::try_deserialize(&mut candy_machine_account.data.as_ref()).unwrap();
    assert_eq!(candy_machine_state.items_redeemed, 1);

    let asset_account = banks_client
        .get_account(asset.pubkey())
        .await
        .expect("get_account")
        .expect("asset_account not found");

    // Add more assertions as needed to verify the asset account data
    assert!(!asset_account.data.is_empty());

    // Verify the metadata URI
    let metadata_uri = "https://devnet.irys.xyz/7xG7g4CNU2AB4WNgbcFBorNg2GTYckdEe9j7FZmBEfM2";
    let config_line = get_config_line(&candy_machine_state, 0, 1).unwrap();
    assert_eq!(config_line.uri, metadata_uri);

}
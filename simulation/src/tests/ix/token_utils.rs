use cpmm_rebalansing::DECIMALS;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::{Account, Mint};

use crate::utils::ix::{context::SimContext, token_utils::TokenUtils};

#[test]
fn test_create_mint_basic() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let decimals = 6;

    let result = TokenUtils::create_mint(&mut ctx, decimals);
    assert!(result.is_ok(), "Create mint should succeed");

    let (mint_pubkey, mint_keypair) = result.unwrap();
    assert_eq!(mint_pubkey, mint_keypair.pubkey());

    // Check if the mint exists in the context
    let account = ctx.svm.get_account(&mint_pubkey);
    assert!(account.is_some(), "Mint account should exist");

    let account = account.unwrap();
    assert_eq!(
        account.owner,
        spl_token::ID,
        "Account should be owned by token program"
    );

    // Check mint data
    let mint_data = Mint::unpack(&account.data).expect("Should be able to unpack mint data");
    assert_eq!(mint_data.decimals, decimals, "Decimals should match");
    assert_eq!(
        mint_data.mint_authority.unwrap(),
        ctx.payer.pubkey(),
        "Mint authority should be payer"
    );
    assert_eq!(mint_data.supply, 0, "Initial supply should be 0");
    assert!(
        mint_data.freeze_authority.is_none(),
        "Freeze authority should be None"
    );
}

#[test]
fn test_create_mint_different_decimals() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    for decimals in [0, 2, 6, 8, 9] {
        let result = TokenUtils::create_mint(&mut ctx, decimals);
        assert!(
            result.is_ok(),
            "Create mint with {} decimals should succeed",
            decimals
        );

        let (mint_pubkey, _) = result.unwrap();
        let account = ctx.svm.get_account(&mint_pubkey).unwrap();
        let mint_data = Mint::unpack(&account.data).unwrap();

        assert_eq!(
            mint_data.decimals, decimals,
            "Decimals should be {}",
            decimals
        );
    }
}

#[test]
fn test_create_mint_insufficient_funds() {
    let mut ctx = SimContext::new();

    // We do not airdrop - payer has 0 lamports

    let result = TokenUtils::create_mint(&mut ctx, 6);

    assert!(result.is_err(), "Create mint should fail without funds");
}

#[test]
fn test_create_token_account_basic() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    // First, create a mint
    let (mint_pubkey, _) = TokenUtils::create_mint(&mut ctx, 6).unwrap();

    let owner = ctx.payer.pubkey();
    let initial_amount = 1000000; // 1 token with 6 decimal places
    let result = TokenUtils::create_token_account(&mut ctx, &mint_pubkey, owner, initial_amount);
    assert!(result.is_ok(), "Create token account should succeed");

    let token_account = result.unwrap();
    let expected_ata = get_associated_token_address(&owner, &mint_pubkey);
    assert_eq!(
        token_account, expected_ata,
        "Should return correct ATA address"
    );

    // Check if the account exists
    let account = ctx.svm.get_account(&token_account);
    assert!(account.is_some(), "Token account should exist");

    let account = account.unwrap();
    assert_eq!(
        account.owner,
        spl_token::ID,
        "Account should be owned by token program"
    );

    // Check token account data
    let token_data =
        Account::unpack(&account.data).expect("Should be able to unpack token account data");
    assert_eq!(token_data.mint, mint_pubkey, "Mint should match");
    assert_eq!(token_data.owner, owner, "Owner should match");
    assert_eq!(token_data.amount, initial_amount, "Amount should match");
}

#[test]
fn test_create_token_account_zero_amount() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let (mint_pubkey, _) = TokenUtils::create_mint(&mut ctx, 6).unwrap();
    let owner = ctx.payer.pubkey();

    let result = TokenUtils::create_token_account(&mut ctx, &mint_pubkey, owner, 0);
    assert!(
        result.is_ok(),
        "Create token account with 0 amount should succeed"
    );

    let token_account = result.unwrap();
    let account = ctx.svm.get_account(&token_account).unwrap();
    let token_data = Account::unpack(&account.data).unwrap();

    assert_eq!(token_data.amount, 0, "Amount should be 0");
}

#[test]
fn test_create_token_account_different_owner() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let (mint_pubkey, _) = TokenUtils::create_mint(&mut ctx, 6).unwrap();
    let different_owner = Keypair::new().pubkey();
    let initial_amount = 500000;

    let result =
        TokenUtils::create_token_account(&mut ctx, &mint_pubkey, different_owner, initial_amount);
    assert!(
        result.is_ok(),
        "Create token account for different owner should succeed"
    );

    let token_account = result.unwrap();
    let expected_ata = get_associated_token_address(&different_owner, &mint_pubkey);
    assert_eq!(token_account, expected_ata);

    let account = ctx.svm.get_account(&token_account).unwrap();
    let token_data = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_data.owner, different_owner,
        "Owner should be different owner"
    );
    assert_eq!(token_data.amount, initial_amount);
}

#[test]
fn test_create_token_account_nonexistent_mint() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let fake_mint = Pubkey::new_unique();
    let owner = ctx.payer.pubkey();

    let result = TokenUtils::create_token_account(&mut ctx, &fake_mint, owner, 1000);
    assert!(
        result.is_err(),
        "Create token account with nonexistent mint should fail"
    );
}

#[test]
fn test_setup_sorted_mints() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let token_pair = TokenUtils::setup_sorted_mints(&mut ctx);

    // Check sorting: mint_a > mint_b
    assert!(
        token_pair.mint_a > token_pair.mint_b,
        "mint_a should be greater than mint_b"
    );

    // Check if both mints exist
    let mint_a_account = ctx.svm.get_account(&token_pair.mint_a);
    let mint_b_account = ctx.svm.get_account(&token_pair.mint_b);

    assert!(mint_a_account.is_some(), "mint_a should exist");
    assert!(mint_b_account.is_some(), "mint_b should exist");

    // Check if these are valid mints
    let mint_a_data = Mint::unpack(&mint_a_account.unwrap().data).unwrap();
    let mint_b_data = Mint::unpack(&mint_b_account.unwrap().data).unwrap();

    assert_eq!(mint_a_data.decimals, DECIMALS, "mint_a should have 9 decimals");
    assert_eq!(mint_b_data.decimals, DECIMALS, "mint_b should have 9 decimals");
}

#[test]
fn test_setup_sorted_mints_multiple_calls() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let pair1 = TokenUtils::setup_sorted_mints(&mut ctx);
    let pair2 = TokenUtils::setup_sorted_mints(&mut ctx);

    // Each call should create different mints
    assert_ne!(pair1.mint_a, pair2.mint_a);
    assert_ne!(pair1.mint_b, pair2.mint_b);
    assert_ne!(pair1.mint_a, pair2.mint_b);
    assert_ne!(pair1.mint_b, pair2.mint_a);

    // But each pair should be sorted
    assert!(pair1.mint_a > pair1.mint_b);
    assert!(pair2.mint_a > pair2.mint_b);
}

#[test]
fn test_multiple_token_accounts_same_mint() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let (mint_pubkey, _) = TokenUtils::create_mint(&mut ctx, 6).unwrap();

    let owner1 = ctx.payer.pubkey();
    let owner2 = Keypair::new().pubkey();
    let owner3 = Keypair::new().pubkey();

    let account1 =
        TokenUtils::create_token_account(&mut ctx, &mint_pubkey, owner1, 1000000).unwrap();

    let account2 =
        TokenUtils::create_token_account(&mut ctx, &mint_pubkey, owner2, 2000000).unwrap();

    let account3 = TokenUtils::create_token_account(&mut ctx, &mint_pubkey, owner3, 0).unwrap();

    // All accounts should be different
    assert_ne!(account1, account2);
    assert_ne!(account2, account3);
    assert_ne!(account1, account3);

    // Check balances
    let acc1_data = Account::unpack(&ctx.svm.get_account(&account1).unwrap().data).unwrap();
    let acc2_data = Account::unpack(&ctx.svm.get_account(&account2).unwrap().data).unwrap();
    let acc3_data = Account::unpack(&ctx.svm.get_account(&account3).unwrap().data).unwrap();

    assert_eq!(acc1_data.amount, 1000000);
    assert_eq!(acc2_data.amount, 2000000);
    assert_eq!(acc3_data.amount, 0);

    // All should have the same mint
    assert_eq!(acc1_data.mint, mint_pubkey);
    assert_eq!(acc2_data.mint, mint_pubkey);
    assert_eq!(acc3_data.mint, mint_pubkey);
}

#[test]
fn test_create_token_account_already_exists() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let (mint_pubkey, _) = TokenUtils::create_mint(&mut ctx, 6).unwrap();
    let owner = ctx.payer.pubkey();

    // Create the account for the first time
    let result1 = TokenUtils::create_token_account(&mut ctx, &mint_pubkey, owner, 1000000);
    assert!(result1.is_ok(), "First creation should succeed");

    // Try to create the same account again
    let result2 = TokenUtils::create_token_account(&mut ctx, &mint_pubkey, owner, 2000000);

    // This may fail or succeed depending on the ATA implementation
    // If it succeeds, check if the balance did not change
    if result2.is_ok() {
        let account = ctx.svm.get_account(&result1.unwrap()).unwrap();
        let token_data = Account::unpack(&account.data).unwrap();
        // The balance should remain original (ATA already existed)
        assert_eq!(token_data.amount, 1000000, "Balance should remain original");
    }
}

#[test]
fn test_rent_exemption() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();

    let initial_balance = ctx.svm.get_account(&ctx.payer.pubkey()).unwrap().lamports;

    let (mint_pubkey, _) = TokenUtils::create_mint(&mut ctx, 6).unwrap();

    let final_balance = ctx.svm.get_account(&ctx.payer.pubkey()).unwrap().lamports;
    let cost = initial_balance - final_balance;

    // Cost should be greater than 0 (rent + fees)
    assert!(cost > 0, "Creating mint should cost lamports");

    // Check if the mint account is rent exempt
    let mint_account = ctx.svm.get_account(&mint_pubkey).unwrap();
    let rent_exempt_minimum = ctx.svm.minimum_balance_for_rent_exemption(Mint::LEN);

    assert!(
        mint_account.lamports >= rent_exempt_minimum,
        "Mint account should be rent exempt"
    );
}

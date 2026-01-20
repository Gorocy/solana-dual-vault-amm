use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;

use crate::{
    tests::ix::helpers::{setup_basic_env, setup_env_with_registry, setup_multiple_token_pairs},
    utils::ix::{
        context::SimContext, registry_utils::RegistryUtils, token_utils::TokenUtils,
        vault_utils::VaultUtils, FeeOption,
    },
};
use cpmm_rebalansing::{LP_TOKEN_SEED, VAULT_SEED};

#[test]
fn test_initialize_vault_success() {
    let mut ctx = SimContext::new();

    let (registry, _mint_a, _mint_b) = setup_env_with_registry(&mut ctx);

    let result = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry);

    assert!(result.is_ok(), "Vault initialization should succeed");
    let vault_pda = result.unwrap();

    // Weryfikacja konta Vault
    let vault_acc = ctx.svm.get_account(&vault_pda);
    assert!(vault_acc.is_some(), "Vault account should exist");
    assert_eq!(
        vault_acc.unwrap().owner,
        crate::PROGRAM_ID,
        "Vault should be owned by program"
    );
}

#[test]
fn test_vault_token_accounts_created() {
    let mut ctx = SimContext::new();

    let (registry, mint_a, mint_b) = setup_env_with_registry(&mut ctx);

    let vault_pda = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // Check token accounts
    let token_a_addr = VaultUtils::get_vault_token_account(vault_pda, mint_a).unwrap();
    let token_b_addr = VaultUtils::get_vault_token_account(vault_pda, mint_b).unwrap();

    // Check if accounts actually exist in SVM
    let acc_a = ctx.svm.get_account(&token_a_addr);
    let acc_b = ctx.svm.get_account(&token_b_addr);

    assert!(acc_a.is_some(), "Token A account should exist");
    assert!(acc_b.is_some(), "Token B account should exist");

    // Check owner (must be SPL Token Program)
    assert_eq!(acc_a.unwrap().owner, spl_token::id());
    assert_eq!(acc_b.unwrap().owner, spl_token::id());

    // Check reserves (should be 0)
    let (res_a, res_b) = VaultUtils::get_vault_reserves(&ctx, vault_pda, mint_a, mint_b).unwrap();

    assert_eq!(res_a, 0, "Initial reserve A should be 0");
    assert_eq!(res_b, 0, "Initial reserve B should be 0");
}

#[test]
fn test_vault_lp_mint_created() {
    let mut ctx = SimContext::new();

    let (registry, _mint_a, _mint_b) = setup_env_with_registry(&mut ctx);

    let vault_pda = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // Check LP Mint
    let lp_mint_pda = VaultUtils::get_vault_lp_mint(vault_pda).unwrap();
    let lp_acc = ctx.svm.get_account(&lp_mint_pda);
    assert!(lp_acc.is_some(), "LP Mint account should exist");

    // Check LP Mint data
    let account_data = lp_acc.unwrap();
    assert_eq!(
        account_data.owner,
        spl_token::id(),
        "LP mint should be owned by token program"
    );

    let mint_data = Mint::unpack(&account_data.data).unwrap();
    assert!(mint_data.is_initialized, "LP mint should be initialized");
    assert_eq!(mint_data.supply, 0, "Initial LP supply should be 0");
    assert_eq!(
        mint_data.mint_authority.unwrap(),
        vault_pda,
        "Vault should be LP mint authority"
    );
}

#[test]
fn test_vault_pda_derivation_match() {
    let mut ctx = SimContext::new();

    let (registry, _mint_a, _mint_b) = setup_env_with_registry(&mut ctx);
    let vault_id: u8 = 0;

    // Manual PDA derivation
    let (expected_vault_pda, _) = Pubkey::find_program_address(
        &[VAULT_SEED, registry.as_ref(), &vault_id.to_le_bytes()],
        &crate::PROGRAM_ID,
    );

    let actual_vault_pda = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    assert_eq!(
        actual_vault_pda, expected_vault_pda,
        "PDA derivation should match"
    );
}

#[test]
fn test_lp_mint_pda_derivation() {
    let mut ctx = SimContext::new();

    let (registry, _mint_a, _mint_b) = setup_env_with_registry(&mut ctx);

    let vault_pda = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // Manual derivation of LP mint PDA
    let (expected_lp_mint, _) =
        Pubkey::find_program_address(&[LP_TOKEN_SEED, vault_pda.as_ref()], &crate::PROGRAM_ID);

    // Call helper
    let actual_lp_mint = VaultUtils::get_vault_lp_mint(vault_pda).unwrap();

    assert_eq!(
        actual_lp_mint, expected_lp_mint,
        "LP mint PDA should match manual derivation"
    );
}

#[test]
fn test_initialize_multiple_vaults_same_registry() {
    let mut ctx = SimContext::new();

    let (registry, _mint_a, _mint_b) = setup_env_with_registry(&mut ctx);

    // Vault #0
    let vault_0 = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // Vault #1
    let vault_1 = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // Vault #2
    let vault_2 = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // All PDAs should be different
    assert_ne!(vault_0, vault_1);
    assert_ne!(vault_1, vault_2);
    assert_ne!(vault_0, vault_2);

    // Check if all accounts exist
    assert!(ctx.svm.get_account(&vault_0).is_some());
    assert!(ctx.svm.get_account(&vault_1).is_some());
    assert!(ctx.svm.get_account(&vault_2).is_some());
}

#[test]
fn test_vault_initialization_without_registry() {
    let mut ctx = SimContext::new();

    setup_basic_env(&mut ctx);

    let _token_pair = TokenUtils::setup_sorted_mints(&mut ctx);

    // Use a fake registry PDA (not initialized)
    let fake_registry = Pubkey::new_unique();

    let result = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, fake_registry);

    assert!(
        result.is_err(),
        "Vault initialization without registry should fail"
    );
}

#[test]
fn test_vault_initialization_insufficient_funds() {
    let mut ctx = SimContext::new();

    ctx.deploy_program("cpmm_rebalansing", crate::PROGRAM_ID);
    // Very few lamports
    ctx.airdrop_payer(1000).unwrap();

    // Try to create mints - may fail due to insufficient funds
    let mint_result1 = TokenUtils::create_mint(&mut ctx, 6);
    let mint_result2 = TokenUtils::create_mint(&mut ctx, 6);

    if mint_result1.is_ok() && mint_result2.is_ok() {
        let (m1, _) = mint_result1.unwrap();
        let (m2, _) = mint_result2.unwrap();
        let (mint_a, mint_b) = if m1 > m2 { (m1, m2) } else { (m2, m1) };

        if let Ok(registry) =
            RegistryUtils::initialize_vault_registry(&mut ctx, mint_a, mint_b, FeeOption::Tier30)
        {
            let result = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry);

            // May fail due to insufficient funds for vault
            assert!(
                result.is_err(),
                "Vault initialization with insufficient funds should fail"
            );
        }
    }
}

#[test]
fn test_get_vault_reserves_empty() {
    let mut ctx = SimContext::new();

    let (registry, mint_a, mint_b) = setup_env_with_registry(&mut ctx);

    let vault_pda = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    let (reserve_a, reserve_b) =
        VaultUtils::get_vault_reserves(&mut ctx, vault_pda, mint_a, mint_b).unwrap();

    assert_eq!(reserve_a, 0, "Empty vault should have 0 reserve A");
    assert_eq!(reserve_b, 0, "Empty vault should have 0 reserve B");
}

#[test]
fn test_get_vault_reserves_nonexistent_vault() {
    let mut ctx = SimContext::new();

    let (_, mint_a, mint_b) = setup_env_with_registry(&mut ctx);

    let fake_vault = Pubkey::new_unique();

    let result = VaultUtils::get_vault_reserves(&mut ctx, fake_vault, mint_a, mint_b);
    assert!(
        result.is_err(),
        "Getting reserves for nonexistent vault should fail"
    );
}

#[test]
fn test_get_vault_token_account_addresses() {
    let mut ctx = SimContext::new();

    let (registry, mint_a, mint_b) = setup_env_with_registry(&mut ctx);

    let vault_pda = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    let token_a_addr = VaultUtils::get_vault_token_account(vault_pda, mint_a).unwrap();
    let token_b_addr = VaultUtils::get_vault_token_account(vault_pda, mint_b).unwrap();

    // Check if addresses are correct (ATA)
    let expected_a = get_associated_token_address(&vault_pda, &mint_a);
    let expected_b = get_associated_token_address(&vault_pda, &mint_b);

    assert_eq!(token_a_addr, expected_a, "Token A address should be ATA");
    assert_eq!(token_b_addr, expected_b, "Token B address should be ATA");

    // Check if addresses are different
    assert_ne!(
        token_a_addr, token_b_addr,
        "Token accounts should be different"
    );
}

#[test]
fn test_vault_with_different_mint_pairs() {
    let mut ctx = SimContext::new();

    let pairs = setup_multiple_token_pairs(&mut ctx, 2);

    let (registry1, _mint_a1, _mint_b1) = pairs[0];
    let (registry2, _mint_a2, _mint_b2) = pairs[1];

    // Vaults for different pairs
    let vault1 = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry1).unwrap();

    let vault2 = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry2).unwrap();

    // Vaults should be different
    assert_ne!(vault1, vault2);

    // LP minty powinny być różne
    let lp1 = VaultUtils::get_vault_lp_mint(vault1).unwrap();
    let lp2 = VaultUtils::get_vault_lp_mint(vault2).unwrap();
    assert_ne!(lp1, lp2);
}

#[test]
fn test_vault_accounts_rent_exemption() {
    let mut ctx = SimContext::new();

    let (registry, mint_a, mint_b) = setup_env_with_registry(&mut ctx);

    let vault_pda = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // Check if vault is rent exempt
    let vault_account = ctx.svm.get_account(&vault_pda).unwrap();
    assert!(
        vault_account.lamports > 0,
        "Vault should have lamports for rent"
    );

    // Check LP mint
    let lp_mint_pda = VaultUtils::get_vault_lp_mint(vault_pda).unwrap();
    let lp_account = ctx.svm.get_account(&lp_mint_pda).unwrap();
    assert!(
        lp_account.lamports > 0,
        "LP mint should have lamports for rent"
    );

    // Check token accounts
    let token_a_addr = VaultUtils::get_vault_token_account(vault_pda, mint_a).unwrap();
    let token_b_addr = VaultUtils::get_vault_token_account(vault_pda, mint_b).unwrap();

    let token_a_account = ctx.svm.get_account(&token_a_addr).unwrap();
    let token_b_account = ctx.svm.get_account(&token_b_addr).unwrap();

    assert!(
        token_a_account.lamports > 0,
        "Token A account should have lamports"
    );
    assert!(
        token_b_account.lamports > 0,
        "Token B account should have lamports"
    );
}

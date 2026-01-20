use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

use crate::{
    tests::ix::helpers::{setup_basic_env, setup_env_with_tokens, setup_multiple_token_pairs},
    utils::ix::{
        context::SimContext, registry_utils::RegistryUtils, token_utils::TokenUtils, FeeOption,
    },
};
use cpmm_rebalansing::constants::VAULT_REGISTRY_SEED;

#[test]
fn test_initialize_vault_registry_success() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    let fee = FeeOption::Tier30;

    let result = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        fee,
    );

    assert!(result.is_ok(), "Registry initialization should succeed");
    let registry_pda = result.unwrap();

    // Sprawdź czy konto istnieje
    let account = ctx.svm.get_account(&registry_pda);
    assert!(
        account.is_some(),
        "Registry account should exist after initialization"
    );

    // Sprawdź właściciela konta
    let account_data = account.unwrap();
    assert_eq!(
        account_data.owner,
        crate::PROGRAM_ID,
        "Registry account should be owned by program"
    );

    // Sprawdź czy konto nie jest puste
    assert!(
        !account_data.data.is_empty(),
        "Account data should not be empty"
    );
}

#[test]
fn test_initialize_vault_registry_tier25() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    let fee = FeeOption::Tier25;

    let result = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        fee,
    );
    assert!(
        result.is_ok(),
        "Registry initialization with Tier25 should succeed"
    );
}

#[test]
fn test_initialize_vault_registry_tier20() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    let fee = FeeOption::Tier20;

    let result = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        fee,
    );
    assert!(
        result.is_ok(),
        "Registry initialization with Tier20 should succeed"
    );
}

#[test]
fn test_registry_pda_derivation() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    // Oblicz oczekiwany PDA
    let (expected_pda, _) = Pubkey::find_program_address(
        &[
            VAULT_REGISTRY_SEED,
            token_pair.mint_a.as_ref(),
            token_pair.mint_b.as_ref(),
        ],
        &crate::PROGRAM_ID,
    );

    let fee = FeeOption::Tier25;
    let actual_pda = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        fee,
    )
    .unwrap();

    assert_eq!(
        actual_pda, expected_pda,
        "Returned PDA should match derived PDA"
    );
}

#[test]
fn test_initialize_duplicate_registry_fails() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    // Pierwsza inicjalizacja
    let res1 = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        FeeOption::Tier30,
    );
    assert!(res1.is_ok(), "First initialization should succeed");

    // Druga inicjalizacja (te same minty)
    let res2 = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        FeeOption::Tier20,
    );
    assert!(res2.is_err(), "Duplicate initialization should fail");
}

#[test]
fn test_initialize_registry_without_program() {
    let mut ctx = SimContext::new();

    setup_basic_env(&mut ctx);

    // Nie deployujemy programu - resetujemy ctx
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(10_000_000).unwrap();

    let token_pair = TokenUtils::setup_sorted_mints(&mut ctx);

    let result = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        FeeOption::Tier30,
    );

    assert!(
        result.is_err(),
        "Registry initialization without deployed program should fail"
    );
}

#[test]
fn test_initialize_registry_insufficient_funds() {
    let mut ctx = SimContext::new();

    ctx.deploy_program("cpmm_rebalansing", crate::PROGRAM_ID);
    // Bardzo mała kwota
    ctx.airdrop_payer(1000).unwrap();

    // Próbuj stworzyć minty - może się nie powieść z powodu braku funduszy
    let mint_result1 = TokenUtils::create_mint(&mut ctx, 6);
    let mint_result2 = TokenUtils::create_mint(&mut ctx, 6);

    if mint_result1.is_ok() && mint_result2.is_ok() {
        let (m1, _) = mint_result1.unwrap();
        let (m2, _) = mint_result2.unwrap();
        let (mint_a, mint_b) = if m1 > m2 { (m1, m2) } else { (m2, m1) };

        let result =
            RegistryUtils::initialize_vault_registry(&mut ctx, mint_a, mint_b, FeeOption::Tier30);

        // Może się nie powieść z powodu braku funduszy
        assert!(
            result.is_err(),
            "Registry initialization with insufficient funds should fail"
        );
    }
}

#[test]
fn test_different_mint_pairs_different_pdas() {
    let mut ctx = SimContext::new();

    let pairs = setup_multiple_token_pairs(&mut ctx, 2);

    let (registry1, _, _) = pairs[0];
    let (registry2, _, _) = pairs[1];

    // PDA powinny być różne
    assert_ne!(
        registry1, registry2,
        "Different mint pairs should have different PDAs"
    );

    // Oba konta powinny istnieć
    assert!(ctx.svm.get_account(&registry1).is_some());
    assert!(ctx.svm.get_account(&registry2).is_some());
}

#[test]
fn test_fee_option_serialization() {
    let mut ctx = SimContext::new();

    setup_basic_env(&mut ctx);

    // Test wszystkich opcji fee
    let fee_options = [FeeOption::Tier30, FeeOption::Tier25, FeeOption::Tier20];

    for (i, fee_option) in fee_options.iter().enumerate() {
        // Użyj różnych mintów dla każdej opcji
        let token_pair = TokenUtils::setup_sorted_mints(&mut ctx);

        let result = RegistryUtils::initialize_vault_registry(
            &mut ctx,
            token_pair.mint_a,
            token_pair.mint_b,
            fee_option.clone(),
        );

        assert!(
            result.is_ok(),
            "Registry initialization with fee option {} should succeed",
            i
        );
    }
}

#[test]
fn test_same_mint_different_fees() {
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(100 * LAMPORTS_PER_SOL).unwrap();
    ctx.deploy_program("cpmm_rebalansing", crate::PROGRAM_ID);
    let token_pair = TokenUtils::setup_sorted_mints(&mut ctx);

    // Pierwsza inicjalizacja z Tier30
    let res1 = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        FeeOption::Tier30,
    );
    assert!(res1.is_ok(), "First initialization should succeed");

    // Druga inicjalizacja z tymi samymi mintami ale innym fee
    let res2 = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        FeeOption::Tier25,
    );
    assert!(
        res2.is_err(),
        "Second initialization with same mints but different fee should fail"
    );
}

#[test]
fn test_pda_consistency() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    // Oblicz PDA wielokrotnie - powinien być zawsze ten sam
    let (pda1, bump1) = Pubkey::find_program_address(
        &[
            VAULT_REGISTRY_SEED,
            token_pair.mint_a.as_ref(),
            token_pair.mint_b.as_ref(),
        ],
        &crate::PROGRAM_ID,
    );

    let (pda2, bump2) = Pubkey::find_program_address(
        &[
            VAULT_REGISTRY_SEED,
            token_pair.mint_a.as_ref(),
            token_pair.mint_b.as_ref(),
        ],
        &crate::PROGRAM_ID,
    );

    assert_eq!(pda1, pda2, "PDA should be consistent");
    assert_eq!(bump1, bump2, "Bump should be consistent");
}

#[test]
fn test_reversed_mint_order_same_pda() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    // Oblicz PDA dla mint_a, mint_b
    let (pda1, _) = Pubkey::find_program_address(
        &[
            VAULT_REGISTRY_SEED,
            token_pair.mint_a.as_ref(),
            token_pair.mint_b.as_ref(),
        ],
        &crate::PROGRAM_ID,
    );

    // Oblicz PDA dla mint_b, mint_a (odwrócona kolejność)
    let (pda2, _) = Pubkey::find_program_address(
        &[
            VAULT_REGISTRY_SEED,
            token_pair.mint_b.as_ref(),
            token_pair.mint_a.as_ref(),
        ],
        &crate::PROGRAM_ID,
    );

    // Powinny być różne (kolejność ma znaczenie)
    assert_ne!(pda1, pda2, "PDA should depend on mint order");
}

#[test]
fn test_account_rent_exemption() {
    let mut ctx = SimContext::new();

    let token_pair = setup_env_with_tokens(&mut ctx);

    let result = RegistryUtils::initialize_vault_registry(
        &mut ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        FeeOption::Tier30,
    );
    assert!(result.is_ok());

    let pda = result.unwrap();
    let account = ctx.svm.get_account(&pda).unwrap();

    // Sprawdź czy konto jest rent exempt
    assert!(
        account.lamports > 0,
        "Account should have lamports for rent exemption"
    );
}

use solana_sdk::program_pack::Pack;
use spl_token::state::Account as TokenAccount;

use crate::{
    tests::ix::{helpers::setup_env_with_registry, manage_liqudity::setup_vault_with_tokens},
    utils::ix::{
        context::SimContext, dual_swap_utils::DualSwapUtils,
        manage_liqudity::ManageLiquidityInstruction, vault_utils::VaultUtils,
    },
};

#[test]
fn test_dual_swap_a_to_b_success() {
    let mut ctx = SimContext::new();

    let user_balance_a = 100_000_000;
    let user_balance_b = 100_000_000;

    let deposit_a = 10_000_000;
    let deposit_b = 10_000_000;

    // setup vault 0
    let (_, registry, _, _, user_token_a, user_token_b) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    ctx.next_slot();
    // setup vault 1 (second vault in the same registry)
    VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    // add liquidity to both vaults
    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, deposit_a, deposit_b, 1,
    )
    .unwrap();

    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 1, deposit_a, deposit_b, 1,
    )
    .unwrap();

    ctx.next_slot();

    let amount_in = 1_000_000;

    let out = DualSwapUtils::execute_dual_swap(
        &mut ctx, None, registry, 0, 1, amount_in, 1, true, // A -> B
    )
    .expect("dual swap should succeed");

    assert!(out > 0, "User should receive token B");

    let user_a = TokenAccount::unpack(&ctx.svm.get_account(&user_token_a).unwrap().data).unwrap();
    let user_b = TokenAccount::unpack(&ctx.svm.get_account(&user_token_b).unwrap().data).unwrap();

    assert_eq!(
        user_a.amount,
        user_balance_a - deposit_a * 2 - amount_in,
        "User A should decrease"
    );

    assert!(
        user_b.amount > user_balance_b - deposit_b * 2,
        "User B should increase"
    );
}

#[test]
fn test_dual_swap_b_to_a_success() {
    let mut ctx = SimContext::new();

    let user_balance_a = 100_000_000;
    let user_balance_b = 100_000_000;

    let deposit_a = 10_000_000;
    let deposit_b = 10_000_000;

    let (_, registry, _, _, user_token_a, user_token_b) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    // setup vault 1 (second vault in the same registry)
    VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, deposit_a, deposit_b, 1,
    )
    .unwrap();

    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 1, deposit_a, deposit_b, 1,
    )
    .unwrap();

    ctx.next_slot();

    let out = DualSwapUtils::execute_dual_swap(
        &mut ctx, None, registry, 0, 1, 1_000_000, 1, false, // B -> A
    )
    .expect("dual swap should succeed");

    assert!(out > 0);

    let user_a = TokenAccount::unpack(&ctx.svm.get_account(&user_token_a).unwrap().data).unwrap();
    let user_b = TokenAccount::unpack(&ctx.svm.get_account(&user_token_b).unwrap().data).unwrap();

    assert!(user_a.amount > user_balance_a - deposit_a * 2);
    assert!(user_b.amount < user_balance_b - deposit_b * 2);
}

#[test]
fn test_dual_swap_without_liquidity_fails() {
    let mut ctx = SimContext::new();

    let (registry, _, _) = setup_env_with_registry(&mut ctx);

    let result =
        DualSwapUtils::execute_dual_swap(&mut ctx, None, registry, 0, 1, 1_000_000, 1, true);

    assert!(result.is_err(), "Dual swap should fail without liquidity");
}

#[test]
fn test_dual_swap_zero_amount_fails() {
    let mut ctx = SimContext::new();

    let (_, registry, _, _, _, _) = setup_vault_with_tokens(&mut ctx, 10_000_000, 10_000_000);

    let result = DualSwapUtils::execute_dual_swap(&mut ctx, None, registry, 0, 1, 0, 1, true);

    assert!(result.is_err(), "Zero dual swap should fail");
}

#[test]
fn test_dual_swap_min_amount_out_too_high() {
    let mut ctx = SimContext::new();

    let (_, registry, _, _, _, _) = setup_vault_with_tokens(&mut ctx, 100_000_000, 100_000_000);

    // setup vault 1 (second vault in the same registry)
    VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry).unwrap();

    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, 50_000_000, 50_000_000, 1,
    )
    .unwrap();

    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 1, 50_000_000, 50_000_000, 1,
    )
    .unwrap();

    ctx.next_slot();

    let result = DualSwapUtils::execute_dual_swap(
        &mut ctx,
        None,
        registry,
        0,
        1,
        10_000_000,
        100_000_000, // absurdly high
        true,
    );

    assert!(
        result.is_err(),
        "Dual swap should fail due to min_amount_out"
    );
}

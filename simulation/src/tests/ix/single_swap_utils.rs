use spl_token::state::Account as TokenAccount;

use crate::{
    tests::ix::{helpers::setup_env_with_registry, manage_liqudity::setup_vault_with_tokens},
    utils::ix::{
        context::SimContext, manage_liqudity::ManageLiquidityInstruction,
        single_swap_utils::SingleSwapUtils,
    },
};
use solana_sdk::program_pack::Pack;

#[test]
fn test_single_swap_a_to_b_success() {
    let mut ctx = SimContext::new();

    let user_balance_a = 100_000_000;
    let user_balance_b = 10_000_000;

    let deposit_a = 5_000_000;
    let deposit_b = 5_000_000;

    let (_, registry, _, _, user_token_a, user_token_b) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    // Dodaj płynność
    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, deposit_a, deposit_b, 1,
    )
    .expect("add liquidity");

    ctx.next_slot();

    let amount_in = 1_000_000;

    let out = SingleSwapUtils::execute_single_swap(
        &mut ctx, None, registry, 0, amount_in, 1, true, // A -> B
    )
    .expect("swap should succeed");

    assert!(out > 0, "User should receive token B");

    let user_a = TokenAccount::unpack(&ctx.svm.get_account(&user_token_a).unwrap().data).unwrap();
    let user_b = TokenAccount::unpack(&ctx.svm.get_account(&user_token_b).unwrap().data).unwrap();

    assert_eq!(
        user_a.amount,
        user_balance_a - deposit_a - amount_in,
        "User A should decrease"
    );

    assert!(
        user_b.amount > user_balance_b - deposit_b,
        "User B should increase"
    );
}

#[test]
fn test_single_swap_b_to_a_success() {
    let mut ctx = SimContext::new();

    let user_balance_a = 10_000_000;
    let user_balance_b = 100_000_000;

    let deposit_a = 5_000_000u64;
    let deposit_b = 5_000_000;
    let (_, registry, _, _, user_token_a, user_token_b) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, deposit_a, deposit_b, 1,
    )
    .expect("add liquidity");

    ctx.next_slot();

    let out = SingleSwapUtils::execute_single_swap(
        &mut ctx, None, registry, 0, 1_000_000, 1, false, // B -> A
    )
    .expect("swap should succeed");

    assert!(out > 0);

    let user_a = TokenAccount::unpack(&ctx.svm.get_account(&user_token_a).unwrap().data).unwrap();
    let user_b = TokenAccount::unpack(&ctx.svm.get_account(&user_token_b).unwrap().data).unwrap();

    assert!(user_a.amount > user_balance_a - deposit_a);
    assert!(user_b.amount < user_balance_b - deposit_b);
}

#[test]
fn test_single_swap_without_liquidity_fails() {
    let mut ctx = SimContext::new();

    let (registry, _, _) = setup_env_with_registry(&mut ctx);

    let result =
        SingleSwapUtils::execute_single_swap(&mut ctx, None, registry, 0, 1_000_000, 1, true);

    assert!(result.is_err(), "Swap should fail without liquidity");
}

#[test]
fn test_single_swap_zero_amount_fails() {
    let mut ctx = SimContext::new();

    let (_, registry, _, _, _, _) = setup_vault_with_tokens(&mut ctx, 10_000_000, 10_000_000);

    let result = SingleSwapUtils::execute_single_swap(&mut ctx, None, registry, 0, 0, 1, true);

    assert!(result.is_err(), "Zero swap should fail");
}

#[test]
fn test_single_swap_min_amount_out_too_high() {
    let mut ctx = SimContext::new();

    let (_, registry, _, _, _, _) = setup_vault_with_tokens(&mut ctx, 100_000_000, 100_000_000);

    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, 50_000_000, 50_000_000, 1,
    )
    .unwrap();

    ctx.next_slot();

    let result = SingleSwapUtils::execute_single_swap(
        &mut ctx,
        None,
        registry,
        0,
        10_000_000,
        100_000_000, // absurdalnie wysokie
        true,
    );

    assert!(result.is_err(), "Swap should fail due to min_amount_out");
}

use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signature::Signer};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account as TokenAccount;

use crate::{
    tests::ix::helpers::{setup_basic_env, setup_env_with_registry},
    utils::ix::{
        context::SimContext, manage_liqudity::ManageLiquidityInstruction, token_utils::TokenUtils,
        vault_utils::VaultUtils,
    },
};

/// Helper: Setup full environment with a vault ready for liquidity management
/// Returns: (Vault PDA, Registry PDA, Mint A, Mint B, User Token A Account, User Token B Account)
pub fn setup_vault_with_tokens(
    ctx: &mut SimContext,
    user_balance_a: u64,
    user_balance_b: u64,
) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let (registry, mint_a, mint_b) = setup_env_with_registry(ctx);
    // Create vault
    let vault_pda =
        VaultUtils::init_vault(ctx, crate::PROGRAM_ID, registry).expect("Failed to create vault");

    // Create user token accounts with balances
    let payer = ctx.payer.pubkey();
    let user_token_a = TokenUtils::create_token_account(ctx, &mint_a, payer, user_balance_a)
        .expect("Failed to create user token A account");

    let user_token_b = TokenUtils::create_token_account(ctx, &mint_b, payer, user_balance_b)
        .expect("Failed to create user token B account");

    (
        vault_pda,
        registry,
        mint_a,
        mint_b,
        user_token_a,
        user_token_b,
    )
}

#[test]
fn test_add_liquidity_to_vault_success() {
    let mut ctx = SimContext::new();

    let user_balance_a = 10_000_000; // 10 tokens (6 decimals)
    let user_balance_b = 20_000_000; // 20 tokens (6 decimals)

    let (vault_pda, registry, mint_a, mint_b, user_token_a, user_token_b) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    let amount_a = 5_000_000; // 5 tokens
    let amount_b = 10_000_000; // 10 tokens

    // Add liquidity
    let result = ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, // vault_index
        amount_a, amount_b, 1,
    );

    assert!(result.is_ok(), "Add liquidity should succeed");

    // Check user balances after adding liquidity
    let user_account_a = ctx.svm.get_account(&user_token_a).unwrap();
    let user_account_b = ctx.svm.get_account(&user_token_b).unwrap();

    let user_token_data_a = TokenAccount::unpack(&user_account_a.data).unwrap();
    let user_token_data_b = TokenAccount::unpack(&user_account_b.data).unwrap();

    assert_eq!(
        user_token_data_a.amount,
        user_balance_a - amount_a,
        "User should have less token A after adding liquidity"
    );
    assert_eq!(
        user_token_data_b.amount,
        user_balance_b - amount_b,
        "User should have less token B after adding liquidity"
    );

    // Check vault reserves
    let (reserve_a, reserve_b) =
        VaultUtils::get_vault_reserves(&mut ctx, vault_pda, mint_a, mint_b).unwrap();

    assert_eq!(reserve_a, amount_a, "Vault should have added token A");
    assert_eq!(reserve_b, amount_b, "Vault should have added token B");
}

#[test]
fn test_add_liquidity_insufficient_balance() {
    let mut ctx = SimContext::new();

    let user_balance_a = 1_000_000; // 1 token
    let user_balance_b = 2_000_000; // 2 tokens

    let (_, registry, _, _, _, _) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    let amount_a = 5_000_000; // 5 tokens - more than user has
    let amount_b = 10_000_000; // 10 tokens - more than user has

    let result = ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, amount_a, amount_b, 1,
    );

    assert!(
        result.is_err(),
        "Add liquidity should fail with insufficient balance"
    );
}

#[test]
fn test_add_liquidity_zero_amounts() {
    let mut ctx = SimContext::new();

    let user_balance_a = 10_000_000;
    let user_balance_b = 20_000_000;

    let (_, registry, _, _, _, _) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    let result = ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, 0, // zero amount A
        0, // zero amount B
        1,
    );

    assert!(
        result.is_err(),
        "Add liquidity should fail with zero amounts"
    );
}

#[test]
fn test_add_liquidity_nonexistent_vault() {
    let mut ctx = SimContext::new();

    let user_balance_a = 10_000_000;
    let user_balance_b = 20_000_000;

    let (_, registry, _, _, _, _) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    let result = ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 99, // nonexistent vault index
        5_000_000, 10_000_000, 1,
    );

    assert!(
        result.is_err(),
        "Add liquidity should fail with nonexistent vault"
    );
}

#[test]
fn test_remove_liquidity_from_vault_success() {
    let mut ctx = SimContext::new();

    let user_balance_a = 20_000_000;
    let user_balance_b = 40_000_000;

    let (vault_pda, registry, _, _, user_token_a, user_token_b) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    let add_amount_a = 10_000_000;
    let add_amount_b = 20_000_000;

    // First, add liquidity
    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx,
        None,
        registry,
        0,
        add_amount_a,
        add_amount_b,
        1,
    )
    .expect("Adding liquidity should succeed");

    ctx.next_slot();

    // Check if user received LP tokens
    let lp_mint_pda = VaultUtils::get_vault_lp_mint(vault_pda).unwrap();
    let user_lp_token = get_associated_token_address(&ctx.payer.pubkey(), &lp_mint_pda);

    let user_lp_account = ctx.svm.get_account(&user_lp_token);
    assert!(
        user_lp_account.is_some(),
        "User should have LP token account"
    );

    let user_lp_data = TokenAccount::unpack(&user_lp_account.unwrap().data).unwrap();
    let lp_balance = user_lp_data.amount;
    assert!(lp_balance > 0, "User should have some LP tokens");

    // Now remove some liquidity
    let remove_lp_amount = lp_balance / 2; // remove half
    let result = ManageLiquidityInstruction::remove_liquidity_from_vault(
        &mut ctx,
        None,
        registry,
        0,
        remove_lp_amount,
        1, // min_amount_a
        1, // min_amount_b
    );

    assert!(result.is_ok(), "Remove liquidity should succeed");

    // Check balances after removing liquidity
    let user_account_a_after = ctx.svm.get_account(&user_token_a).unwrap();
    let user_account_b_after = ctx.svm.get_account(&user_token_b).unwrap();

    let user_token_data_a_after = TokenAccount::unpack(&user_account_a_after.data).unwrap();
    let user_token_data_b_after = TokenAccount::unpack(&user_account_b_after.data).unwrap();

    assert!(
        user_token_data_a_after.amount > user_balance_a - add_amount_a,
        "User should have recovered some token A"
    );
    assert!(
        user_token_data_b_after.amount > user_balance_b - add_amount_b,
        "User should have recovered some token B"
    );
}

#[test]
fn test_remove_liquidity_insufficient_lp_tokens() {
    let mut ctx = SimContext::new();

    let user_balance_a = 20_000_000;
    let user_balance_b = 40_000_000;

    let (_, registry, _, _, _, _) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    // Try to remove liquidity without adding it first
    let result = ManageLiquidityInstruction::remove_liquidity_from_vault(
        &mut ctx, None, registry, 0, 1_000_000, // LP amount user does not have
        1, 1,
    );

    assert!(
        result.is_err(),
        "Remove liquidity should fail without LP tokens"
    );
}

#[test]
fn test_remove_liquidity_zero_amount() {
    let mut ctx = SimContext::new();

    let user_balance_a = 20_000_000;
    let user_balance_b = 40_000_000;

    let (_, registry, _, _, _, _) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    // Add some liquidity
    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, 10_000_000, 20_000_000, 1,
    )
    .expect("Adding liquidity should succeed");

    // Try to remove zero LP tokens
    let result = ManageLiquidityInstruction::remove_liquidity_from_vault(
        &mut ctx, None, registry, 0, 0, // zero LP amount
        1, 1,
    );

    assert!(
        result.is_err(),
        "Remove liquidity should fail with zero LP amount"
    );
}

#[test]
fn test_multiple_liquidity_operations() {
    let mut ctx = SimContext::new();

    let user_balance_a = 100_000_000;
    let user_balance_b = 200_000_000;

    let (vault_pda, registry, mint_a, mint_b, user_token_a, _) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    // First add liquidity
    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, 20_000_000, 40_000_000, 1,
    )
    .expect("First add liquidity should succeed");

    ctx.next_slot();
    
    // Second add liquidity
    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, 30_000_000, 60_000_000, 1,
    )
    .expect("Second add liquidity should succeed");

    // Check vault reserves
    let (reserve_a, reserve_b) =
        VaultUtils::get_vault_reserves(&mut ctx, vault_pda, mint_a, mint_b).unwrap();

    assert_eq!(reserve_a, 50_000_000, "Vault should have total token A");
    assert_eq!(reserve_b, 100_000_000, "Vault should have total token B");

    // Check LP balance
    let lp_mint_pda = VaultUtils::get_vault_lp_mint(vault_pda).unwrap();
    let user_lp_token = get_associated_token_address(&ctx.payer.pubkey(), &lp_mint_pda);
    let user_lp_account = ctx.svm.get_account(&user_lp_token).unwrap();
    let user_lp_data = TokenAccount::unpack(&user_lp_account.data).unwrap();
    let total_lp_balance = user_lp_data.amount;

    // Remove some liquidity
    let remove_amount = total_lp_balance / 4; // remove 1/4
    ManageLiquidityInstruction::remove_liquidity_from_vault(
        &mut ctx,
        None,
        registry,
        0,
        remove_amount,
        1,
        1,
    )
    .expect("Remove liquidity should succeed");

    // Check vault reserves after removal
    let (final_reserve_a, final_reserve_b) =
        VaultUtils::get_vault_reserves(&mut ctx, vault_pda, mint_a, mint_b).unwrap();

    assert!(
        final_reserve_a < 50_000_000,
        "Vault reserve A should decrease"
    );
    assert!(
        final_reserve_b < 100_000_000,
        "Vault reserve B should decrease"
    );

    let final_user_account_a = ctx.svm.get_account(&user_token_a).unwrap();

    let final_user_token_data_a = TokenAccount::unpack(&final_user_account_a.data).unwrap();

    // User should have more than after adding liquidity, but less than at the beginning
    assert!(
        final_user_token_data_a.amount < user_balance_a,
        "User should have less than initial"
    );
    assert!(
        final_user_token_data_a.amount > user_balance_a - 50_000_000,
        "User should have recovered some"
    );
}

#[test]
fn test_add_liquidity_with_wrong_registry() {
    let mut ctx = SimContext::new();

    setup_basic_env(&mut ctx);

    let fake_registry = Pubkey::new_unique();

    let result = ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx,
        None,
        fake_registry, // wrong registry
        0,
        1_000_000,
        2_000_000,
        1,
    );

    assert!(
        result.is_err(),
        "Add liquidity should fail with wrong registry"
    );
}

#[test]
fn test_remove_liquidity_high_min_amounts() {
    let mut ctx = SimContext::new();

    let user_balance_a = 50_000_000;
    let user_balance_b = 100_000_000;

    let (_, registry, _, _, _, _) =
        setup_vault_with_tokens(&mut ctx, user_balance_a, user_balance_b);

    // Add some liquidity
    ManageLiquidityInstruction::add_liquidity_to_vault(
        &mut ctx, None, registry, 0, 20_000_000, 40_000_000, 1,
    )
    .expect("Adding liquidity should succeed");

    // Try to remove with very high min_amounts
    let result = ManageLiquidityInstruction::remove_liquidity_from_vault(
        &mut ctx,
        None,
        registry,
        0,
        1_000,       // small amount of LP
        50_000_000,  // very high min_amount_a
        100_000_000, // very high min_amount_b
    );

    assert!(
        result.is_err(),
        "Remove liquidity should fail with high min amounts"
    );
}

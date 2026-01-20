//! # Single Vault Swap Execution Logic
//!
//! Handles calculation and execution flow for single vault swaps.

use super::{accounts_single::SwapSingle, transfers_single};
use crate::{utils::SwapUtilsSingle, MINIMUM_LIQUIDITY};
use anchor_lang::prelude::*;

#[error_code]
pub enum ExecutionError {
    #[msg("Liquidity too low - minimum liquidity constraint violated")]
    LiquidityTooLow,
    #[msg("Slippage limit exceeded")]
    SlippageExceeded,
    #[msg("Math overflow")]
    MathOverflow,
}

/// Executes a single CPMM swap
pub fn execute_single_swap<'info>(
    accounts: &SwapSingle<'info>,
    amount_in: u64,
    min_amount_out: u64,
    fee_rate: u64,
) -> Result<u64> {
    // Returns amount_out

    // 1. Calculate Output
    let (amount_out, fee) = SwapUtilsSingle::calculate_output(
        amount_in,
        accounts.vault_token_in.amount,
        accounts.vault_token_out.amount,
        fee_rate,
    )?;

    // 2. Check Slippage
    require!(
        amount_out >= min_amount_out,
        ExecutionError::SlippageExceeded
    );

    // 3. Execute Transfers

    // User -> Vault (Input + Fee)
    // Fee stays in the vault (increases reserve_in without increasing output capability immediately for this trade)
    transfers_single::transfer_from_user_to_vault(accounts, amount_in)?;

    // Vault -> User (Output)
    transfers_single::transfer_from_vault_to_user(accounts, amount_out)?;

    // 4. Verify Minimum Liquidity Residue
    let remaining_liquidity = accounts.vault_token_out.amount.saturating_sub(amount_out);
    require!(
        remaining_liquidity >= MINIMUM_LIQUIDITY,
        ExecutionError::LiquidityTooLow
    );

    msg!(
        "Single Swap: {} In -> {} Out (Fee: {})",
        amount_in,
        amount_out,
        fee
    );

    Ok(amount_out)
}

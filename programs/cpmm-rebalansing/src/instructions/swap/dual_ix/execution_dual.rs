//! # Dual Vault Swap Execution Logic
//!
//! Orchestrates the physical token transfers based on the
//! mathematical plan calculated by SwapUtilsVirtualVault.

use anchor_lang::prelude::*;

use super::accounts_dual::SwapDualDeterministic;
use super::transfers_dual::{transfer_from_user_to_vault, transfer_from_vault_to_user};
use crate::{SafeMath, MINIMUM_LIQUIDITY};

#[error_code]
pub enum ExecutionError {
    #[msg("Liquidity too low - minimum liquidity constraint violated")]
    LiquidityTooLow,
    #[msg("Slippage limit exceeded")]
    SlippageExceeded,
}

/// Executes the calculated routing plan.
///
/// This function does NOT perform calculations. It trusts the inputs provided
/// from the VirtualVault logic and performs physical transfers.
pub fn execute_planned_swap<'info>(
    accounts: &SwapDualDeterministic<'info>,
    net_inputs: (u64, u64), // (in_v1, in_v2) - amount used for swap
    outputs: (u64, u64),    // (out_v1, out_v2)
    fees: (u64, u64),       // (fee_v1, fee_v2)
    min_amount_out: u64,
) -> Result<()> {
    let (net_in_v1, net_in_v2) = net_inputs;
    let (out_v1, out_v2) = outputs;
    let (fee_v1, fee_v2) = fees;

    let total_out = out_v1.safe_add(out_v2)?;

    // 1. Check Slippage
    require!(
        total_out >= min_amount_out,
        ExecutionError::SlippageExceeded
    );

    // 2. Execute Vault 1 Transfers
    // Total Input to V1 = Net Input + Fee assigned to V1
    let total_in_v1 = net_in_v1.safe_add(fee_v1)?;

    if total_in_v1 > 0 {
        transfer_from_user_to_vault(accounts, total_in_v1, true)?;
    }

    if out_v1 > 0 {
        transfer_from_vault_to_user(accounts, out_v1, true)?;

        // Liquidity check after withdrawal
        let remaining_out = accounts.vault1_token_out.amount.saturating_sub(out_v1);
        require!(
            remaining_out >= MINIMUM_LIQUIDITY,
            ExecutionError::LiquidityTooLow
        );
    }

    // 3. Execute Vault 2 Transfers
    // Total Input to V2 = Net Input + Fee assigned to V2
    let total_in_v2 = net_in_v2.safe_add(fee_v2)?;

    if total_in_v2 > 0 {
        transfer_from_user_to_vault(accounts, total_in_v2, false)?;
    }

    if out_v2 > 0 {
        transfer_from_vault_to_user(accounts, out_v2, false)?;

        // Liquidity check after withdrawal
        let remaining_out = accounts.vault2_token_out.amount.saturating_sub(out_v2);
        require!(
            remaining_out >= MINIMUM_LIQUIDITY,
            ExecutionError::LiquidityTooLow
        );
    }

    Ok(())
}

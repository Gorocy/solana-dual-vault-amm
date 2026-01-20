//! # Dual Vault Swap Handler
//!
//! Main instruction handler for dual vault swaps.

pub mod accounts_dual;
pub mod execution_dual;
pub mod transfers_dual;

pub use accounts_dual::*;
pub use execution_dual::*;

use crate::utils::SwapUtilsVirtualVault;
use crate::VaultAssets;
use anchor_lang::prelude::*;

/// Main function swap for dual vault
pub fn swap_dual_deterministic(
    ctx: Context<SwapDualDeterministic>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<()> {
    let accounts = ctx.accounts;

    // 1. Validate inputs
    accounts.validate(amount_in, minimum_amount_out)?;

    let fee_rate = accounts.vault_registry.fee.normal();
    
    // 2. Snapshot Vault Assets
    // We snapshot balances BEFORE any transfers to perform correct math
    let v1_assets = VaultAssets::new(
        accounts.vault1_token_in.amount,
        accounts.vault1_token_out.amount,
    );
    let v2_assets = VaultAssets::new(
        accounts.vault2_token_in.amount,
        accounts.vault2_token_out.amount,
    );

    // 3. Calculate everything using Virtual Vault Math (Rebalancing & Liquidity Fees)
    let (total_out, net_inputs, outputs, fees) =
        SwapUtilsVirtualVault::calculate_routing_and_output(amount_in, fee_rate, &v1_assets, &v2_assets)?;

    // 4. Execute Physical Transfers based on calculation
    execute_planned_swap(accounts, net_inputs, outputs, fees, minimum_amount_out)?;

    // 5. Emit Event
    emit!(SwapDualEvent {
        user: accounts.user.key(),
        vault_registry: accounts.vault_registry.key(),
        vault1: accounts.vault1.key(),
        vault2: accounts.vault2.key(),
        token_in: accounts.token_in_mint.key(),
        token_out: accounts.token_out_mint.key(),
        amount_in,
        amount_out: total_out,
        fee1: fees.0,
        fee2: fees.1,
        input_vault1: net_inputs.0,
        input_vault2: net_inputs.1,
    });

    msg!(
        "Swap completed: {} In -> {} Out (Fee: {}, {})",
        amount_in,
        total_out,
        fees.0,
        fees.1
    );

    Ok(())
}

#[event]
pub struct SwapDualEvent {
    pub user: Pubkey,
    pub vault_registry: Pubkey,
    pub vault1: Pubkey,
    pub vault2: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee1: u64,
    pub fee2: u64,
    pub input_vault1: u64,
    pub input_vault2: u64,
}

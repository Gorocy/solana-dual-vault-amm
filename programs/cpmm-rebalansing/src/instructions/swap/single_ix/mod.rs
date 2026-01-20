//! # Single Vault Swap Handler
//!
//! Main instruction handler for single vault standard swaps.

pub mod accounts_single;
pub mod execution_single;
pub mod transfers_single;

pub use accounts_single::*;
use anchor_lang::prelude::*;

/// Main function for standard single vault swap
pub fn swap_single(
    ctx: Context<SwapSingle>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<()> {
    let accounts = ctx.accounts;

    // 1. Validate inputs
    accounts.validate(amount_in, minimum_amount_out)?;

    // 2. Determine fee rate (assuming fee is stored in Vault or Registry)
    // For this example, let's say we fetch it from a hypothetical field or constant
    // In real app: accounts.vault.fee_rate or accounts.registry.fee_rate
    let fee_rate = accounts.vault_registry.fee.normal(); // 0.3% example (30/10000)

    // 3. Execute Swap
    let amount_out =
        execution_single::execute_single_swap(accounts, amount_in, minimum_amount_out, fee_rate)?;

    // 4. Emit Event
    emit!(SwapSingleEvent {
        user: accounts.user.key(),
        vault: accounts.vault.key(),
        token_in: accounts.token_in_mint.key(),
        token_out: accounts.token_out_mint.key(),
        amount_in,
        amount_out,
    });

    Ok(())
}

#[event]
pub struct SwapSingleEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
}

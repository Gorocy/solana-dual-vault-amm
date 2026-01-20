pub mod constants;
pub mod instructions;
pub mod state;
pub mod utils;

use anchor_lang::prelude::*;

pub use constants::*;
use instructions::*;
pub use state::*;
pub use utils::*;

declare_id!("4p6EZrNaZyrABpyJGdb9JGLVUj8wbq5w81TBw5TDfdNB");

#[program]
pub mod cpmm_rebalansing {
    use super::*;

    // ============================================================================
    // INITIALIZE INSTRUCTIONS
    // ============================================================================

    /// Initialize a new vault registry with two tokens (mint_a and mint_b)
    /// and configure the fee structure
    pub fn initialize_vault_registry(
        ctx: Context<InitializeVaultRegistry>,
        mint_a: Pubkey,
        mint_b: Pubkey,
        fee: FeeOption,
    ) -> Result<()> {
        crate::instructions::initialize::registry::initialize_vault_registry(
            ctx, mint_a, mint_b, fee,
        )
    }

    /// Initialize a vault for a specific vault registry
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize::vault_init::initialize_vault(ctx)
    }

    // ============================================================================
    // LIQUIDITY INSTRUCTIONS
    // ============================================================================

    /// Add liquidity to the vault by depositing both tokens
    /// Returns LP tokens proportional to the deposit
    ///
    /// Parameters:
    /// - amount_a: Amount of token A to deposit
    /// - amount_b: Amount of token B to deposit  
    /// - min_lp_amount: Minimum LP tokens to receive (slippage protection)
    pub fn add_liquidity(
        ctx: Context<ManageLiquidity>,
        amount_a: u64,
        amount_b: u64,
        min_lp_amount: u64,
    ) -> Result<()> {
        instructions::liquidity::add::add_liquidity(ctx, amount_a, amount_b, min_lp_amount)
    }

    /// Remove liquidity from the vault by burning LP tokens
    /// Receives proportional amounts of both tokens back
    ///
    /// Parameters:
    /// - lp_amount: Number of LP tokens to burn
    /// - min_amount_a: Minimum token A to receive (slippage protection)
    /// - min_amount_b: Minimum token B to receive (slippage protection)
    pub fn remove_liquidity(
        ctx: Context<ManageLiquidity>,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        instructions::liquidity::remove::remove_liquidity(
            ctx,
            lp_amount,
            min_amount_a,
            min_amount_b,
        )
    }

    // ============================================================================
    // SWAP INSTRUCTIONS
    // ============================================================================

    /// Execute a swap across dual vaults with rebalancing
    /// Routes the input through both vaults optimally based on liquidity
    ///
    /// Parameters:
    /// - amount_in: Amount of token_in to swap
    /// - minimum_amount_out: Minimum token_out to receive (slippage protection)
    pub fn swap_dual_deterministic(
        ctx: Context<SwapDualDeterministic>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<()> {
        instructions::swap::dual_ix::swap_dual_deterministic(ctx, amount_in, minimum_amount_out)
    }

    /// Execute a swap on a single vault using constant product formula (x*y=k)
    /// Simple swap without rebalancing across multiple vaults
    ///
    /// Parameters:
    /// - amount_in: Amount of token_in to swap
    /// - minimum_amount_out: Minimum token_out to receive (slippage protection)
    #[cfg(feature = "test_single")]
    pub fn swap_single(
        ctx: Context<SwapSingle>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<()> {
        instructions::swap::single_ix::swap_single(ctx, amount_in, minimum_amount_out)
    }
}

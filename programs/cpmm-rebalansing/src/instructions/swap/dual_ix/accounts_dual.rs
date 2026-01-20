//! # Dual Vault Swap Accounts Structure
//!
//! Contains account definitions and validation logic for dual vault swaps

use anchor_lang::prelude::*;

#[cfg(feature = "spl-token-only")]
use anchor_spl::token::Token;

#[cfg(feature = "token-2022")]
use anchor_spl::token_interface::TokenInterface;

use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::{
    constants::{VAULT_REGISTRY_SEED, VAULT_SEED},
    state::{Vault, VaultRegistry},
};

/// Accounts required for dual vault swap
#[derive(Accounts)]
pub struct SwapDualDeterministic<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [VAULT_REGISTRY_SEED, vault_registry.token_a.as_ref(), vault_registry.token_b.as_ref()],
        bump = vault_registry.bump
    )]
    pub vault_registry: Account<'info, VaultRegistry>,

    #[account(
        seeds = [VAULT_SEED, vault_registry.key().as_ref(), vault1.vault_index.to_le_bytes().as_ref()],
        bump = vault1.bump,
        constraint = vault1.registry == vault_registry.key() @ DualSwapValidationError::InvalidVaultRegistry,
        constraint = vault2.vault_index != vault1.vault_index @ DualSwapValidationError::SameVaultSelected,
    )]
    pub vault1: Account<'info, Vault>,

    #[account(
        seeds = [VAULT_SEED, vault_registry.key().as_ref(), vault2.vault_index.to_le_bytes().as_ref()],
        bump = vault2.bump,
        constraint = vault2.registry == vault_registry.key() @ DualSwapValidationError::InvalidVaultRegistry,
        constraint = vault2.vault_index != vault1.vault_index @ DualSwapValidationError::SameVaultSelected
    )]
    pub vault2: Account<'info, Vault>,

    #[account(
        mut,
        token::mint = token_in_mint,
        token::authority = vault1,
    )]
    pub vault1_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_out_mint,
        token::authority = vault1,
    )]
    pub vault1_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_in_mint,
        token::authority = vault2,
    )]
    pub vault2_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_out_mint,
        token::authority = vault2,
    )]
    pub vault2_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_in_mint,
        token::authority = user,
    )]
    pub user_token_in: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_out_mint,
        token::authority = user,
    )]
    pub user_token_out: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_in_mint: InterfaceAccount<'info, Mint>,
    pub token_out_mint: InterfaceAccount<'info, Mint>,

    pub clock: Sysvar<'info, Clock>,
    #[cfg(feature = "spl-token-only")]
    pub token_program: Program<'info, Token>,

    #[cfg(feature = "token-2022")]
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> SwapDualDeterministic<'info> {
    pub fn validate(&self, amount_in: u64, minimum_amount_out: u64) -> Result<()> {
        require!(amount_in > 0, DualSwapValidationError::InvalidAmountIn);
        require!(
            minimum_amount_out > 0,
            DualSwapValidationError::InvalidMinimumAmountOut
        );

        require!(
            self.user_token_in.amount >= amount_in,
            DualSwapValidationError::InsufficientBalance
        );

        // Validate mints match registry
        require!(
            ((self.token_in_mint.key() == self.vault_registry.token_a
                && self.token_out_mint.key() == self.vault_registry.token_b)
                || (self.token_in_mint.key() == self.vault_registry.token_b
                    && self.token_out_mint.key() == self.vault_registry.token_a)),
            DualSwapValidationError::InvalidTokenMint
        );

        // Validate deterministic selection
        let is_valid = crate::is_valid_vault_pair_for_slot(
            self.vault1.vault_index,
            self.vault2.vault_index,
            self.clock.slot,
            self.vault_registry.vault_count,
        )?;

        require!(
            is_valid,
            DualSwapValidationError::InvalidDeterministicSelection
        );

        Ok(())
    }
}

#[error_code]
pub enum DualSwapValidationError {
    #[msg("Invalid amount in")]
    InvalidAmountIn,

    #[msg("Invalid minimum amount out")]
    InvalidMinimumAmountOut,

    #[msg("Insufficient balance")]
    InsufficientBalance,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Provided vault pair does not match deterministic selection")]
    InvalidDeterministicSelection,

    #[msg("Cannot select the same vault twice")]
    SameVaultSelected,

    #[msg("Invalid vault registry")]
    InvalidVaultRegistry,
}

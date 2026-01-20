//! # Single Vault Swap Accounts Structure
//!
//! Contains account definitions and validation logic for single vault swaps

use anchor_lang::prelude::*;
#[cfg(feature = "spl-token-only")]
use anchor_spl::token::Token;
    #[cfg(not(feature ="spl-token-only"))]
use anchor_spl::token_interface::TokenInterface;
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::{constants::VAULT_SEED, state::Vault, VaultRegistry, VAULT_REGISTRY_SEED};

#[derive(Accounts)]
pub struct SwapSingle<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [VAULT_REGISTRY_SEED, vault_registry.token_a.as_ref(), vault_registry.token_b.as_ref()],
        bump = vault_registry.bump
    )]
    pub vault_registry: Account<'info, VaultRegistry>,

    #[account(
        seeds = [
            VAULT_SEED,
            vault_registry.key().as_ref(),
            vault.vault_index.to_le_bytes().as_ref()
        ],
        constraint = vault.registry == vault_registry.key() @ SwapValidationError::InvalidVaultRegistry,
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        token::mint = token_in_mint,
        token::authority = vault,
    )]
    pub vault_token_in: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_out_mint,
        token::authority = vault,
    )]
    pub vault_token_out: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_in_mint,
        token::authority = user,
    )]
    pub user_token_in: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_out_mint,
        token::authority = user,
    )]
    pub user_token_out: InterfaceAccount<'info, TokenAccount>,

    pub token_in_mint: InterfaceAccount<'info, Mint>,
    pub token_out_mint: InterfaceAccount<'info, Mint>,
    #[cfg(feature = "spl-token-only")]
    pub token_program: Program<'info, Token>,

    #[cfg(not(feature ="spl-token-only"))]
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> SwapSingle<'info> {
    pub fn validate(&self, amount_in: u64, minimum_amount_out: u64) -> Result<()> {
        require!(amount_in > 0, SwapValidationError::InvalidAmountIn);
        require!(
            minimum_amount_out > 0,
            SwapValidationError::InvalidMinimumAmountOut
        );

        require!(
            self.user_token_in.amount >= amount_in,
            SwapValidationError::InsufficientBalance
        );

        require!(
            self.vault_token_in.amount > 0 && self.vault_token_out.amount > 0,
            SwapValidationError::InsufficientBalance
        );

        require!(
            (self.token_in_mint.key() == self.vault_registry.token_a
                && self.token_out_mint.key() == self.vault_registry.token_b)
                || (self.token_in_mint.key() == self.vault_registry.token_b
                    && self.token_out_mint.key() == self.vault_registry.token_a),
            SwapValidationError::InvalidTokenMints
        );

        Ok(())
    }
}

#[error_code]
pub enum SwapValidationError {
    #[msg("Invalid amount in")]
    InvalidAmountIn,
    #[msg("Invalid minimum amount out")]
    InvalidMinimumAmountOut,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Invalid vault registry")]
    InvalidVaultRegistry,
    #[msg("Token mints do not match vault registry")]
    InvalidTokenMints,
}

pub mod add;
pub mod remove;

pub use add::*;
pub use remove::*;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    constants::VAULT_REGISTRY_SEED,
    state::{Vault, VaultRegistry},
    LP_TOKEN_SEED, VAULT_SEED,
};

#[derive(Accounts)]
pub struct ManageLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [
            VAULT_REGISTRY_SEED,
            vault_registry.token_a.as_ref(),
            vault_registry.token_b.as_ref(),
        ],
        bump = vault_registry.bump
    )]
    pub vault_registry: Account<'info, VaultRegistry>,

    #[account(
        seeds = [
            VAULT_SEED,
            vault_registry.key().as_ref(),
            vault.vault_index.to_le_bytes().as_ref()
        ],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: This is the mint_a from vault_registry
    #[account(address = vault_registry.token_a)]
    pub mint_a: Box<Account<'info, Mint>>,

    /// CHECK: This is the mint_b from vault_registry
    #[account(address = vault_registry.token_b)]
    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_token_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_token_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [LP_TOKEN_SEED, vault.key().as_ref()],
        bump = vault.lp_bump
    )]
    pub mint_lp: Box<Account<'info, Mint>>,

    #[cfg(feature = "spl-token-only")]
    pub token_program: Program<'info, Token>,

    #[cfg(not(feature = "spl-token-only"))]
    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    
    pub system_program: Program<'info, System>,
}

impl<'info> ManageLiquidity<'info> {
    pub fn validate_add_liqudity(&self, amount_a: u64, amount_b: u64) -> Result<()> {
        require!(amount_a > 0, ErrorManageLiquidity::InvalidAmountA);
        require!(amount_b > 0, ErrorManageLiquidity::InvalidAmountB);
        require!(
            self.user_token_a.amount >= amount_a,
            ErrorManageLiquidity::InsufficientBalanceA
        );
        require!(
            self.user_token_b.amount >= amount_b,
            ErrorManageLiquidity::InsufficientBalanceB
        );

        Ok(())
    }

    pub fn validate_remove_liqudity(
        &self,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        require!(lp_amount > 0, ErrorManageLiquidity::InvalidLpAmount);
        require!(
            self.user_lp_token.amount >= lp_amount,
            ErrorManageLiquidity::InsufficientLpBalance
        );

        require!(
            self.vault_token_a.amount >= min_amount_a && self.vault_token_b.amount >= min_amount_b,
            ErrorManageLiquidity::InsufficientVaultLiquidity
        );

        Ok(())
    }
}

#[error_code]
pub enum ErrorManageLiquidity {
    #[msg("Invalid amount A")]
    InvalidAmountA,

    #[msg("Invalid amount B")]
    InvalidAmountB,

    #[msg("Insufficient balance A")]
    InsufficientBalanceA,

    #[msg("Insufficient balance B")]
    InsufficientBalanceB,

    #[msg("Invalid LP amount")]
    InvalidLpAmount,

    #[msg("Insufficient LP token balance")]
    InsufficientLpBalance,

    #[msg("Insufficient vault liquidity")]
    InsufficientVaultLiquidity,
}

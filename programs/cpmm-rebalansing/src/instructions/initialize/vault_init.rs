use std::u8;

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;

#[cfg(feature = "spl-token-only")]
use anchor_spl::token::{Mint, Token, TokenAccount};

#[cfg(feature = "token-2022")]
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    constants::VAULT_REGISTRY_SEED,
    state::{Vault, VaultRegistry},
    VAULT_SEED,
};

use crate::{DECIMALS, LP_TOKEN_SEED};

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [VAULT_REGISTRY_SEED, vault_registry.token_a.as_ref(), vault_registry.token_b.as_ref()],
        bump = vault_registry.bump,
        constraint = vault_registry.vault_count < u8::MAX @ ErrorInitializeVault::MaxVaultsReached,
    )]
    pub vault_registry: Account<'info, VaultRegistry>,

    #[account(
        init,
        payer = payer,
        space = 8 + Vault::INIT_SPACE,
        seeds = [VAULT_SEED, vault_registry.key().as_ref(), vault_registry.vault_count.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: This is the mint_a from vault_registry
    #[account(address = vault_registry.token_a)]
    pub mint_a: Box<Account<'info, Mint>>,

    /// CHECK: This is the mint_b from vault_registry
    #[account(address = vault_registry.token_b)]
    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = payer,
        seeds = [LP_TOKEN_SEED, vault.key().as_ref()],
        bump,
        mint::decimals = DECIMALS,
        mint::authority = vault,
        mint::freeze_authority = vault,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    pub vault_token_b: Box<Account<'info, TokenAccount>>,

    #[cfg(feature = "spl-token-only")]
    pub token_program: Program<'info, Token>,

    #[cfg(feature = "token-2022")]
    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
    let vault_registry: &mut Account<'_, VaultRegistry> = &mut ctx.accounts.vault_registry;
    let vault = &mut ctx.accounts.vault;

    // Initialize vault with index only (registry relationship through seeds)
    vault.vault_index = vault_registry.vault_count;
    vault.bump = ctx.bumps.vault;
    vault.lp_bump = ctx.bumps.mint_lp;
    vault.registry = vault_registry.key();
    // Update registry
    vault_registry.vault_count += 1;

    msg!("Vault initialized!");
    msg!("Vault: {}", vault.key());
    msg!("Vault Index: {}", vault.vault_index);
    msg!("Vault Token A: {}", ctx.accounts.vault_token_a.key());
    msg!("Vault Token B: {}", ctx.accounts.vault_token_b.key());

    Ok(())
}

#[error_code]
pub enum ErrorInitializeVault {
    #[msg("Maximum number of vaults reached")]
    MaxVaultsReached,
}

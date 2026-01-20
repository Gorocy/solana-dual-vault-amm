use anchor_lang::prelude::*;

#[cfg(feature = "spl-token-only")]
use anchor_spl::token::Token;

#[cfg(not(feature ="spl-token-only"))]
use anchor_spl::token_interface::TokenInterface;

use crate::{constants::VAULT_REGISTRY_SEED, state::VaultRegistry, FeeOption};

#[derive(Accounts)]
#[instruction(mint_a: Pubkey, mint_b: Pubkey)]
pub struct InitializeVaultRegistry<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + VaultRegistry::INIT_SPACE,
        seeds = [VAULT_REGISTRY_SEED, mint_a.as_ref(), mint_b.as_ref()],
        bump
    )]
    pub vault_registry: Account<'info, VaultRegistry>,

    #[cfg(feature = "spl-token-only")]
    pub token_program: Program<'info, Token>,

    #[cfg(not(feature ="spl-token-only"))]
    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_vault_registry(
    ctx: Context<InitializeVaultRegistry>,
    mint_a: Pubkey,
    mint_b: Pubkey,
    fee: FeeOption,
) -> Result<()> {
    // Ensure mint_a key is greater than mint_b to maintain canonical order
    require!(
        mint_a > mint_b,
        ErrorInitializeVaultRegistry::InvalidMintOrder
    );

    let vault_registry = &mut ctx.accounts.vault_registry;

    // Initialize registry with token pair info
    vault_registry.token_a = mint_a;
    vault_registry.token_b = mint_b;
    vault_registry.fee = fee;
    vault_registry.bump = ctx.bumps.vault_registry;
    vault_registry.vault_count = 0;

    msg!("Vault Registry initialized!");
    msg!("Token A: {}", vault_registry.token_a);
    msg!("Token B: {}", vault_registry.token_b);
    msg!("Fee normal (bps): {}", vault_registry.fee.normal());
    msg!("Fee normal (bps): {}", vault_registry.fee.discount());

    Ok(())
}

#[error_code]
pub enum ErrorInitializeVaultRegistry {
    #[msg("Invalid mint order")]
    InvalidMintOrder,
}

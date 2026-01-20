//! # Single Vault Token Transfer Logic
//!
//! Handles token transfers between user and a single vault.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_checked, TransferChecked};

use crate::VAULT_SEED;

use super::accounts_single::SwapSingle;

/// Transfer token from user to the vault
pub fn transfer_from_user_to_vault<'info>(accounts: &SwapSingle<'info>, amount: u64) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    transfer_checked(
        CpiContext::new(
            accounts.token_program.to_account_info(),
            TransferChecked {
                from: accounts.user_token_in.to_account_info(),
                mint: accounts.token_in_mint.to_account_info(),
                to: accounts.vault_token_in.to_account_info(),
                authority: accounts.user.to_account_info(),
            },
        ),
        amount,
        accounts.token_in_mint.decimals,
    )?;

    Ok(())
}

/// Transfer token from vault to the user
pub fn transfer_from_vault_to_user<'info>(accounts: &SwapSingle<'info>, amount: u64) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    let vault_registry_key = accounts.vault_registry.key();
    let vault_seeds = &[
        VAULT_SEED,
        vault_registry_key.as_ref(),
        &accounts.vault.vault_index.to_le_bytes(),
        &[accounts.vault.bump],
    ];
    let signer_seeds = &[&vault_seeds[..]];

    transfer_checked(
        CpiContext::new_with_signer(
            accounts.token_program.to_account_info(),
            TransferChecked {
                from: accounts.vault_token_out.to_account_info(),
                mint: accounts.token_out_mint.to_account_info(),
                to: accounts.user_token_out.to_account_info(),
                authority: accounts.vault.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
        accounts.token_out_mint.decimals,
    )?;

    Ok(())
}

//! # Dual Vault Token Transfer Logic
//!
//! Handles all basic token transfers between user and vaults via CPI.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_checked, TransferChecked};

use super::accounts_dual::SwapDualDeterministic;

/// Transfer token from user to vault
pub fn transfer_from_user_to_vault<'info>(
    accounts: &SwapDualDeterministic<'info>,
    amount: u64,
    vault_1: bool, // true for vault1, false for vault2
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    let (user_account, vault_account, mint_account) = match vault_1 {
        true => (
            &accounts.user_token_in,
            &accounts.vault1_token_in,
            &accounts.token_in_mint,
        ),
        false => (
            &accounts.user_token_in,
            &accounts.vault2_token_in,
            &accounts.token_in_mint,
        ),
    };

    transfer_checked(
        CpiContext::new(
            accounts.token_program.to_account_info(),
            TransferChecked {
                from: user_account.to_account_info(),
                mint: mint_account.to_account_info(),
                to: vault_account.to_account_info(),
                authority: accounts.user.to_account_info(),
            },
        ),
        amount,
        mint_account.decimals,
    )?;

    Ok(())
}

/// Transfer token from vault to user
pub fn transfer_from_vault_to_user<'info>(
    accounts: &SwapDualDeterministic<'info>,
    amount: u64,
    vault_1: bool, // true for vault1, false for vault2
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    let vault_registry_key = accounts.vault_registry.key();

    let (vault_account, user_account, mint_account, vault_authority, vault_bump, vault_index) =
        match vault_1 {
            true => (
                &accounts.vault1_token_out,
                &accounts.user_token_out,
                &accounts.token_out_mint,
                &accounts.vault1,
                accounts.vault1.bump,
                accounts.vault1.vault_index,
            ),
            false => (
                &accounts.vault2_token_out,
                &accounts.user_token_out,
                &accounts.token_out_mint,
                &accounts.vault2,
                accounts.vault2.bump,
                accounts.vault2.vault_index,
            ),
        };

    let vault_seeds = &[
        crate::constants::VAULT_SEED,
        vault_registry_key.as_ref(),
        &[vault_index],
        &[vault_bump],
    ];
    let signer_seeds = &[&vault_seeds[..]];

    transfer_checked(
        CpiContext::new_with_signer(
            accounts.token_program.to_account_info(),
            TransferChecked {
                from: vault_account.to_account_info(),
                mint: mint_account.to_account_info(),
                to: user_account.to_account_info(),
                authority: vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
        mint_account.decimals,
    )?;

    Ok(())
}

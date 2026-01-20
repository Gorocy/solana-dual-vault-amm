use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, Transfer};

use crate::{
    instructions::{InstructionExecutionError, ManageLiquidity},
    utils::DepositUtils,
    VAULT_SEED,
};

pub fn add_liquidity(
    ctx: Context<ManageLiquidity>,
    amount_a: u64,
    amount_b: u64,
    min_lp_amount: u64,
) -> Result<()> {
    ctx.accounts.validate_add_liqudity(amount_a, amount_b)?;

    let accounts = &ctx.accounts;

    // Calculate LP tokens to mint based on deposit amounts
    let (lp_tokens, actual_amount_a, actual_amount_b) = DepositUtils::calculate_deposit(
        amount_a,
        amount_b,
        accounts.vault_token_a.amount,
        accounts.vault_token_b.amount,
        accounts.mint_lp.supply,
    )?;

    // Validate slippage protection
    require!(
        lp_tokens >= min_lp_amount,
        InstructionExecutionError::SlippageExceeded
    );

    // Transfer token A from user to vault
    token::transfer(
        CpiContext::new(
            accounts.token_program.to_account_info(),
            Transfer {
                from: accounts.user_token_a.to_account_info(),
                to: accounts.vault_token_a.to_account_info(),
                authority: accounts.user.to_account_info(),
            },
        ),
        actual_amount_a,
    )?;

    // Transfer token B from user to vault
    token::transfer(
        CpiContext::new(
            accounts.token_program.to_account_info(),
            Transfer {
                from: accounts.user_token_b.to_account_info(),
                to: accounts.vault_token_b.to_account_info(),
                authority: accounts.user.to_account_info(),
            },
        ),
        actual_amount_b,
    )?;

    // Mint LP tokens to user
    let vault_registry_key = accounts.vault_registry.key();
    let vault_seeds = &[
        VAULT_SEED,
        vault_registry_key.as_ref(),
        &accounts.vault.vault_index.to_le_bytes(),
        &[accounts.vault.bump],
    ];
    let signer_seeds = &[&vault_seeds[..]];

    token::mint_to(
        CpiContext::new_with_signer(
            accounts.token_program.to_account_info(),
            MintTo {
                mint: accounts.mint_lp.to_account_info(),
                to: accounts.user_lp_token.to_account_info(),
                authority: accounts.vault.to_account_info(),
            },
            signer_seeds,
        ),
        lp_tokens,
    )?;

    // Emit event
    emit!(AddLiquidityEvent {
        user: accounts.user.key(),
        vault: accounts.vault.key(),
        vault_registry: accounts.vault_registry.key(),
        amount_a: actual_amount_a,
        amount_b: actual_amount_b,
        lp_minted: lp_tokens,
    });

    msg!("Liquidity added!");
    msg!("Vault: {}", accounts.vault.key());
    msg!(
        "Tokens: A={}, B={}, LP={}",
        actual_amount_a,
        actual_amount_b,
        lp_tokens
    );

    Ok(())
}

#[event]
pub struct AddLiquidityEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub vault_registry: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
    pub lp_minted: u64,
}

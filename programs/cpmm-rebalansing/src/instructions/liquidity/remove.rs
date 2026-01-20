use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Transfer};

use crate::{
    instructions::{InstructionExecutionError, ManageLiquidity},
    utils::WithdrawUtils,
    VAULT_SEED,
};

pub fn remove_liquidity(
    ctx: Context<ManageLiquidity>,
    lp_amount: u64,
    min_amount_a: u64,
    min_amount_b: u64,
) -> Result<()> {
    let accounts = ctx.accounts;

    accounts.validate_remove_liqudity(lp_amount, min_amount_a, min_amount_b)?;

    // Calculate amounts to withdraw based on LP tokens burned
    let (amount_a, amount_b) = WithdrawUtils::calculate_withdrawal(
        lp_amount,
        accounts.vault_token_a.amount,
        accounts.vault_token_b.amount,
        accounts.mint_lp.supply,
    )?;

    // Validate slippage protection
    require!(
        amount_a >= min_amount_a && amount_b >= min_amount_b,
        InstructionExecutionError::SlippageExceeded
    );

    // Burn LP tokens from user
    token::burn(
        CpiContext::new(
            accounts.token_program.to_account_info(),
            Burn {
                mint: accounts.mint_lp.to_account_info(),
                from: accounts.user_lp_token.to_account_info(),
                authority: accounts.user.to_account_info(),
            },
        ),
        lp_amount,
    )?;

    // Transfer token A from vault to user
    let vault_registry_key = accounts.vault_registry.key();
    let vault_seeds = &[
        VAULT_SEED,
        vault_registry_key.as_ref(),
        &accounts.vault.vault_index.to_le_bytes(),
        &[accounts.vault.bump],
    ];
    let signer_seeds = &[&vault_seeds[..]];

    if amount_a > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                accounts.token_program.to_account_info(),
                Transfer {
                    from: accounts.vault_token_a.to_account_info(),
                    to: accounts.user_token_a.to_account_info(),
                    authority: accounts.vault.to_account_info(),
                },
                signer_seeds,
            ),
            amount_a,
        )?;
    }

    // Transfer token B from vault to user
    if amount_b > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                accounts.token_program.to_account_info(),
                Transfer {
                    from: accounts.vault_token_b.to_account_info(),
                    to: accounts.user_token_b.to_account_info(),
                    authority: accounts.vault.to_account_info(),
                },
                signer_seeds,
            ),
            amount_b,
        )?;
    }

    // Emit event
    emit!(RemoveLiquidityEvent {
        user: accounts.user.key(),
        vault: accounts.vault.key(),
        vault_registry: accounts.vault_registry.key(),
        lp_amount,
        amount_a,
        amount_b,
    });

    msg!("Liquidity removed!");
    msg!("Vault: {}", accounts.vault.key());
    msg!(
        "LP tokens burned: {}, Tokens: A={}, B={}",
        lp_amount,
        amount_a,
        amount_b
    );

    Ok(())
}

#[event]
pub struct RemoveLiquidityEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub vault_registry: Pubkey,
    pub lp_amount: u64,
    pub amount_a: u64,
    pub amount_b: u64,
}

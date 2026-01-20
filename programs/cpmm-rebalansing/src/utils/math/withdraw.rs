use super::SafeMath;
use anchor_lang::prelude::*;

pub struct WithdrawUtils;

impl WithdrawUtils {
    /// Calculate the amount of tokens to return when burning LP tokens
    /// Returns (amount_a, amount_b)
    pub fn calculate_withdrawal(
        lp_tokens: u64,
        reserve_a: u64,
        reserve_b: u64,
        lp_supply: u64,
    ) -> Result<(u64, u64)> {
        require!(lp_tokens > 0, WithdrawError::InvalidAmount);
        require!(lp_tokens <= lp_supply, WithdrawError::InsufficientLiquidity);
        require!(lp_supply > 0, WithdrawError::InvalidReserve);
        require!(
            reserve_a > 0 || reserve_b > 0,
            WithdrawError::InvalidReserve
        );

        let supply = lp_supply as u128;

        // Formula: amount = (lp_tokens * reserve) / lp_supply
        // Using SafeMath trait for cleaner syntax and overflow protection

        let amount_a = (lp_tokens as u128)
            .safe_mul(reserve_a as u128)?
            .safe_div(supply)? as u64;

        let amount_b = (lp_tokens as u128)
            .safe_mul(reserve_b as u128)?
            .safe_div(supply)? as u64;

        // Sanity check
        require!(
            amount_a <= reserve_a && amount_b <= reserve_b,
            WithdrawError::InvalidCalculation
        );

        Ok((amount_a, amount_b))
    }
}

#[error_code]
pub enum WithdrawError {
    #[msg("Invalid withdrawal amount")]
    InvalidAmount,

    #[msg("Not enough LP tokens in supply")]
    InsufficientLiquidity,

    #[msg("Invalid reserve state")]
    InvalidReserve,

    #[msg("Withdrawal amount too small (rounded to zero)")]
    InsufficientWithdrawal,

    #[msg("Invalid withdrawal calculation")]
    InvalidCalculation,
}

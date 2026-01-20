use crate::integer_sqrt;

use super::SafeMath;
use anchor_lang::prelude::*;

/// Utilities for deposit operations in the AMM
pub struct DepositUtils;

impl DepositUtils {
    /// Calculate the amount of LP tokens to mint for a deposit
    /// Returns (lp_tokens_to_mint, actual_amount_a, actual_amount_b)
    pub fn calculate_deposit(
        desired_amount_a: u64,
        desired_amount_b: u64,
        reserve_a: u64,
        reserve_b: u64,
        lp_supply: u64,
    ) -> Result<(u64, u64, u64)> {
        require!(
            desired_amount_a > 0 && desired_amount_b > 0,
            ErrorDeposit::InsufficientInputAmount
        );

        let (optimal_amount_a, optimal_amount_b) = if lp_supply == 0 {
            // Initial deposit: use desired amounts as-is
            (desired_amount_a, desired_amount_b)
        } else {
            // If lp_supply != 0, reserves must be > 0
            require!(reserve_a > 0 && reserve_b > 0, ErrorDeposit::InvalidReserve);

            // Subsequent deposits: calculate optimal amounts to maintain pool ratio
            Self::calculate_optimal_deposit(
                desired_amount_a,
                desired_amount_b,
                reserve_a,
                reserve_b,
            )?
        };

        let lp_tokens = Self::calculate_lp_tokens_to_mint(
            optimal_amount_a,
            optimal_amount_b,
            reserve_a,
            reserve_b,
            lp_supply,
        )?;

        require!(lp_tokens > 0, ErrorDeposit::InsufficientInputAmount);
        Ok((lp_tokens, optimal_amount_a, optimal_amount_b))
    }

    /// Calculate the amount of LP tokens to mint when depositing liquidity
    /// For initial liquidity: LP = sqrt(amount_a * amount_b)
    /// For subsequent deposits: LP = min(amount_a * total_lp / reserve_a, amount_b * total_lp / reserve_b)
    pub fn calculate_lp_tokens_to_mint(
        amount_a: u64,
        amount_b: u64,
        reserve_a: u64,
        reserve_b: u64,
        lp_supply: u64,
    ) -> Result<u64> {
        if lp_supply == 0 {
            // Initial liquidity: use geometric mean = sqrt(amount_a * amount_b)
            // Using efficient integer square root instead of PreciseNumber
            let product = (amount_a as u128).safe_mul(amount_b as u128)?;

            let lp_tokens = integer_sqrt(product);

            Ok(lp_tokens)
        } else {
            // Subsequent liquidity: proportional to existing reserves
            // Formula: (amount * lp_supply) / reserve
            // We use u128 and SafeMath trait for overflow protection

            let supply = lp_supply as u128;

            let lp_from_a = (amount_a as u128)
                .safe_mul(supply)?
                .safe_div(reserve_a as u128)? as u64;

            let lp_from_b = (amount_b as u128)
                .safe_mul(supply)?
                .safe_div(reserve_b as u128)? as u64;

            // Return the minimum of the two calculated LP amounts
            Ok(if lp_from_a < lp_from_b {
                lp_from_a
            } else {
                lp_from_b
            })
        }
    }

    /// Calculate optimal deposit amounts to maintain pool ratio
    fn calculate_optimal_deposit(
        desired_a: u64,
        desired_b: u64,
        reserve_a: u64,
        reserve_b: u64,
    ) -> Result<(u64, u64)> {
        let res_a = reserve_a as u128;
        let res_b = reserve_b as u128;

        // Calculate optimal amounts based on current pool ratio

        // optimal_b = (desired_a * reserve_b) / reserve_a
        let optimal_b_for_a = (desired_a as u128).safe_mul(res_b)?.safe_div(res_a)? as u64;

        if optimal_b_for_a <= desired_b {
            Ok((desired_a, optimal_b_for_a))
        } else {
            // optimal_a = (desired_b * reserve_a) / reserve_b
            let optimal_a_for_b = (desired_b as u128).safe_mul(res_a)?.safe_div(res_b)? as u64;

            Ok((optimal_a_for_b, desired_b))
        }
    }
}

#[error_code]
pub enum ErrorDeposit {
    #[msg("Insufficient input amount")]
    InsufficientInputAmount,

    #[msg("Invalid Reserve")]
    InvalidReserve,
}

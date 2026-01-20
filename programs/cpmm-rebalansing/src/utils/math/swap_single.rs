use super::SafeMath;
use crate::FEE_DENOMINATOR;
use anchor_lang::prelude::*;

pub struct SwapUtilsSingle;

impl SwapUtilsSingle {
    /// CPMM: dy = (y * dx_net) / (x + dx_net)
    pub fn calculate_output(
        amount_in_raw: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_rate: u64,
    ) -> Result<(u64, u64)> {
        let fee_amount = (amount_in_raw as u128)
            .safe_mul(fee_rate as u128)?
            .safe_div_ceil(FEE_DENOMINATOR as u128)?;

        let amount_in_net = (amount_in_raw as u128).safe_sub(fee_amount)?;

        // ===== 2. CPMM =====
        let numerator = (amount_in_net).safe_mul(reserve_out as u128)?;

        let denominator = (reserve_in as u128).safe_add(amount_in_net)?;

        let amount_out = numerator.safe_div(denominator)? as u64;

        Ok((amount_out, fee_amount as u64))
    }
}

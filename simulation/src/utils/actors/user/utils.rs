use crate::{
    error::{ResultSimulation, SimulationError},
    utils::{
        actors::{
            account::SimUser, SwapResult,
            TradingRoundStats,
        },
        ix::{
            context::SimContext
        },
    },
};
use rand::{rngs::StdRng, Rng};

pub use crate::utils::actors::user::behavior::UserBehaviorConfig;


pub(super) struct ManagerUtils;

impl ManagerUtils {
    pub(super) fn pick_direction_and_balance(
        rng: &mut StdRng,
        config: &UserBehaviorConfig,
        user: &SimUser,
        ctx: &SimContext,
    ) -> Option<(bool, u64)> {
        let is_a_to_b = rng.random::<f64>() < config.a_to_b_probability;
        let balance = if is_a_to_b {
            user.get_balance_a(&ctx.svm)
        } else {
            user.get_balance_b(&ctx.svm)
        };

        if balance == 0 {
            None
        } else {
            Some((is_a_to_b, balance))
        }
    }

    pub(super) fn sample_amount(
        rng: &mut StdRng,
        config: &UserBehaviorConfig,
        available_balance: u64,
    ) -> Option<u64> {
        let swap_percentage: f64 =
            rng.random_range(config.min_swap_amount_percentage..=config.max_swap_amount_percentage);
        let amount_calc = ((available_balance as f64) * swap_percentage).round() as u64;
        let amount_clamped = amount_calc.clamp(1, available_balance);
        if amount_clamped == 0 {
            None
        } else {
            Some(amount_clamped)
        }
    }

    pub(super) fn write_swap_compute(
        writer: &mut crate::output::SwapComputeCsvWriter,
        slot: u64,
        swap_result: &SwapResult,
    ) -> ResultSimulation<()> {
        let vault_1 = swap_result.vault_indices.first().copied();
        let vault_2 = swap_result.vault_indices.get(1).copied();
        writer
            .write_swap(
                slot,
                swap_result.success,
                vault_1,
                vault_2,
                swap_result.is_a_to_b,
                swap_result.amount_in,
                swap_result.amount_out,
                swap_result.compute_units,
            )
            .map_err(|e| {
                SimulationError::SystemError(format!("Failed to write compute swap record: {e}"))
            })
    }

    pub(super) fn pick_single_vault_index(
        rng: &mut StdRng,
        vault_count: u8,
    ) -> Option<usize> {
        if vault_count == 0 {
            None
        } else {
            Some(rng.random_range(0..vault_count) as usize)
        }
    }

    pub(super) fn pick_dual_vault_index(
        rng: &mut StdRng,
        vault_pairs: &[(u8, u8)],
    ) -> Option<usize> {
        if vault_pairs.is_empty() {
            None
        } else {
            Some(rng.random_range(0..vault_pairs.len()))
        }
    }

    pub(super) fn record_swap_stats(stats: &mut TradingRoundStats, swap_result: &SwapResult) {
        stats.successful_swaps += 1;
        if swap_result.is_a_to_b {
            stats.total_volume_a += swap_result.amount_in;
        } else {
            stats.total_volume_b += swap_result.amount_in;
        }

        match swap_result.vault_indices.len() {
            1 => stats.single_vault_swaps += 1,
            2 => stats.dual_vault_swaps += 1,
            _ => {}
        }
    }
}

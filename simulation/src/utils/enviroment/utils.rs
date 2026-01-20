use anyhow::Result;
use tracing::info;

use crate::{
    output::{CsvWriter, SwapComputeCsvWriter},
    utils::{
        actors::{
            account::SimUser,
            user::{user_batch::create_sim_users, user_manager::UserBehaviorConfig},
            SwapStrategy,
        },
        args::config_simulation::SimulationConfig,
        ix::context::SimContext,
    },
};
use solana_sdk::pubkey::Pubkey;

pub(crate) fn create_users(
    config: &SimulationConfig,
    ctx: &mut SimContext,
    mint_a: Pubkey,
    mint_b: Pubkey,
) -> Vec<SimUser> {
    info!("  Creating {} simulated users...", config.user_count);

    let users = create_sim_users(
        ctx,
        mint_a,
        mint_b,
        config.user_count,
        config.balance_a_min..=config.balance_a_max,
        config.balance_b_min..=config.balance_b_max,
        config.simulation_seed,
    );

    info!("  Created {} users", users.len());
    users
}

pub(crate) fn user_behavior(config: &SimulationConfig, strategy: SwapStrategy) -> UserBehaviorConfig {
    UserBehaviorConfig {
        min_swap_amount_percentage: config.min_swap_percentage,
        max_swap_amount_percentage: config.max_swap_percentage,
        swap_frequency: config.swap_frequency,
        a_to_b_probability: config.user_a_to_b_probability,
        strategy,
    }
}

pub(crate) fn create_writers(
    config: &SimulationConfig,
) -> Result<(CsvWriter, SwapComputeCsvWriter)> {
    let csv_writer = CsvWriter::new(&config.output_file, config.vault_count)?;
    let swap_compute_writer = SwapComputeCsvWriter::new(&config.compute_csv_file, config.user_count, config.arbitrageur_count, config.swap_frequency)?;
    Ok((csv_writer, swap_compute_writer))
}

pub(crate) fn write_initial_reserves(
    writer: &mut CsvWriter,
    market_price: f64,
    reserves: &[crate::utils::stats::VaultReserve],
    arb_balances: Option<(u64, u64)>,
    user_balances: Option<(u64, u64)>,
) -> Result<()> {
    let reserve_tuples: Vec<(u64, u64)> = reserves
        .iter()
        .map(|r| (r.reserve_a, r.reserve_b))
        .collect();

    writer.write_reserves(0, market_price, &reserve_tuples, arb_balances, user_balances)?;
    info!("Initial vault reserves recorded");
    Ok(())
}

// derive_compute_csv_path retained for compatibility with older outputs; unused in refactored runner.
#[allow(dead_code)]
fn derive_compute_csv_path(base: &str, suffix: &str) -> String {
    if let Some(stripped) = base.strip_suffix(".csv") {
        format!("{}{}.csv", stripped, suffix)
    } else {
        format!("{}{}.csv", base, suffix)
    }
}

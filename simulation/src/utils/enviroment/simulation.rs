use anyhow::Result;
use solana_sdk::native_token::LAMPORTS_PER_SOL;

use crate::{
    output::{CsvWriter, SwapComputeCsvWriter},
    utils::{
        actors::user::user_manager::UserManager, args::config_simulation::SimulationConfig, enviroment::utils::write_initial_reserves, stats::SimulationStats
    },
};

use super::{
    env_driver::SimulationEnvDriver, manager_driver::ArbitrageManagerDriver,
};

/// Flush buffers to CSV every N slots to reduce I/O overhead
pub const FLUSH_INTERVAL_SLOTS: u64 = 1000;

#[derive(Debug, Default)]
pub struct SlotResult {
    pub successful_swaps: u64,
    pub failed_swaps: u64,
}

#[derive(Clone)]
struct SlotData {
    slot: u64,
    market_price: f64,
    reserves: Vec<(u64, u64)>,
    arb_balances: Option<(u64, u64)>,
    user_balances: Option<(u64, u64)>,
}

/// Core simulation runner struct
pub struct Simulation<M>
where
    M: ArbitrageManagerDriver,
    M::Env: SimulationEnvDriver,
{
    env: M::Env,
    user_manager: UserManager,
    arbitrage_manager: Option<M>,
    csv_writer: CsvWriter,
    swap_compute_writer: SwapComputeCsvWriter,
    config: SimulationConfig,
    slot_data_buffer: Vec<SlotData>,
}

impl<M> Simulation<M>
where
    M: ArbitrageManagerDriver,
    M::Env: SimulationEnvDriver,
{
    pub fn new(
        env: M::Env,
        user_manager: UserManager,
        arbitrage_manager: Option<M>,
        csv_writer: CsvWriter,
        swap_compute_writer: SwapComputeCsvWriter,
        config: SimulationConfig,
    ) -> Self {
        Self {
            env,
            user_manager,
            arbitrage_manager,
            csv_writer,
            swap_compute_writer,
            config,
            slot_data_buffer: Vec::with_capacity(FLUSH_INTERVAL_SLOTS as usize),
        }
    }

    pub fn execute_slot(&mut self, slot: u64) -> Result<SlotResult> {
        let mut result = SlotResult::default();

        let vault_pairs = self
            .env
            .slot_setup(slot, &self.config, &mut self.user_manager)?;

        if self.config.enable_arbitrageurs {
            if let Some(manager) = self.arbitrage_manager.as_mut() {
                if let Some(pairs) = vault_pairs.as_ref() {
                    manager.set_shared_state(pairs);
                }

                if !manager.is_empty() {
                    self.env.advance_price();
                    manager.execute_round(&mut self.env, slot)?;
                }
            }
        }

        let registry = self.env.registry();
        match self.user_manager.execute_random_trading_round_with_compute(
            self.env.ctx_mut(),
            registry,
            self.config.vault_count,
            slot,
            Some(&mut self.swap_compute_writer),
        ) {
            Ok(stats) => {
                result.successful_swaps = stats.successful_swaps as u64;
                result.failed_swaps = stats.failed_swaps as u64;
            }
            Err(e) => {
                tracing::warn!("Trading round failed at slot {}: {:?}", slot, e);
                result.failed_swaps += 1;
            }
        }

        
        // Collect data for this slot into buffer
        let reserves = self.env.collect_vault_reserves()?;
        let reserve_tuples: Vec<(u64, u64)> = reserves
            .iter()
            .map(|r| (r.reserve_a, r.reserve_b))
            .collect();

        let market_price = self.env.market_price();

        let arb_balances = self
            .arbitrage_manager
            .as_ref()
            .map(|m| m.total_balances(&self.env.ctx().svm));

        // Conditionally collect user balances based on feature flag
        #[cfg(feature = "track_user_balances")]
        let user_balances = Some(self.user_manager.total_user_balances(self.env.ctx()));
        
        #[cfg(not(feature = "track_user_balances"))]
        let user_balances = None;
        
        self.slot_data_buffer.push(SlotData {
            slot,
            market_price,
            reserves: reserve_tuples,
            arb_balances,
            user_balances,
        });

        self.env.ctx_mut().next_slot();
        
        Ok(result)
    }
    
    fn flush_buffer(&mut self) -> Result<()> {
        for data in &self.slot_data_buffer {
            self.csv_writer.write_reserves(
                data.slot,
                data.market_price,
                &data.reserves,
                data.arb_balances,
                data.user_balances,
            )?;
        }
        self.slot_data_buffer.clear();
        Ok(())
    }

    pub fn run(mut self) -> Result<SimulationStats> {
        self.config.validate()?;

        let total_value_in_a = |balances: (u64, u64), price: f64| -> Option<f64> {
            if price <= 0.0 {
                return None;
            }

            Some(balances.0 as f64 + balances.1 as f64 / price)
        };

        let initial_market_price = self.env.market_price();

        let initial_reserves = self.env.collect_vault_reserves()?;
        let mut stats = SimulationStats::new(initial_reserves.clone());

        let initial_arb_balances = self
            .arbitrage_manager
            .as_ref()
            .map(|m| m.total_balances(&self.env.ctx().svm));

        let initial_arb_value_in_a = initial_arb_balances
            .and_then(|balances| total_value_in_a(balances, initial_market_price));

        let initial_user_balances = self.user_manager.total_user_balances(self.env.ctx());
        stats.initial_user_balances = Some(initial_user_balances);

        write_initial_reserves(
            &mut self.csv_writer,
            initial_market_price,
            &initial_reserves,
            initial_arb_balances,
            Some(initial_user_balances),
        )?;

        for slot in 1..=self.config.max_slots {
            let result = self.execute_slot(slot)?;
            stats.successful_swaps += result.successful_swaps;
            stats.failed_swaps += result.failed_swaps;

            if slot % FLUSH_INTERVAL_SLOTS == 0 {
                // Flush buffered data to CSV every FLUSH_INTERVAL_SLOTS slots
                self.flush_buffer()?;                
                self.swap_compute_writer.flush_buffer()?;                
                tracing::info!(
                    "Progress: slot {}/{} ({:.1}%)",
                    slot,
                    self.config.max_slots,
                    (slot as f64 / self.config.max_slots as f64) * 100.0
                );

                // Reset lamport balances every FLUSH_INTERVAL_SLOTS slots to prevent exhaustion
                let reset_lamports = 100 * LAMPORTS_PER_SOL;
                
                if let Err(e) = self.user_manager.reset_lamport_balances(self.env.ctx_mut(), reset_lamports) {
                    tracing::warn!("Failed to reset user lamport balances at slot {}: {:?}", slot, e);
                }
                
                if let Some(manager) = self.arbitrage_manager.as_mut() {
                    if let Err(e) = manager.reset_lamport_balances(self.env.ctx_mut(), reset_lamports) {
                        tracing::warn!("Failed to reset arbitrage bot lamport balances at slot {}: {:?}", slot, e);
                    }
                }

                self.env.ctx_mut().airdrop_payer(1000 * LAMPORTS_PER_SOL)?;
                if let Some(manager) = self.arbitrage_manager.as_ref() {
                    let balances = manager.total_balances(&self.env.ctx().svm);
                    let price = self.env.market_price();

                    if let Some(current_value_a) = total_value_in_a(balances, price) {
                        let profit_a = initial_arb_value_in_a
                            .map(|initial| current_value_a - initial)
                            .unwrap_or(0.0);

                        let profit_pct = initial_arb_value_in_a
                            .filter(|initial| *initial > 0.0)
                            .map(|initial| (profit_a / initial) * 100.0);

                        tracing::info!(
                            "Arbitrageur funds at slot {}: A={} B={} | total_in_A={:.1} | profit_in_A={:.1} | profit_pct={:.4}%",
                            slot,
                            balances.0 / LAMPORTS_PER_SOL,
                            balances.1 / LAMPORTS_PER_SOL,
                            current_value_a / LAMPORTS_PER_SOL as f64,
                            profit_a / LAMPORTS_PER_SOL as f64,
                            profit_pct.unwrap_or(0.0)
                        );
                    } else {
                        tracing::warn!(
                            "Skipping arbitrageur funds log at slot {} due to non-positive market price {}",
                            slot,
                            price
                        );
                    }
                }
            }
        }

        // Flush any remaining buffered data
        if !self.slot_data_buffer.is_empty() {
            self.flush_buffer()?;
        }

        stats.total_slots = self.config.max_slots;
        stats.final_reserves = self.env.collect_vault_reserves()?;
        stats.arbitrage_profits = self.arbitrage_manager.as_ref().map(|m| m.stats());
        stats.final_user_balances = Some(self.user_manager.total_user_balances(self.env.ctx()));

        // Write final state to CSV
        let final_reserves = stats.final_reserves.clone();
        let final_reserve_tuples: Vec<(u64, u64)> = final_reserves
            .iter()
            .map(|r| (r.reserve_a, r.reserve_b))
            .collect();
        
        self.csv_writer.write_reserves(
            self.config.max_slots + 1,
            self.env.market_price(),
            &final_reserve_tuples,
            self.arbitrage_manager.as_ref().map(|m| m.total_balances(&self.env.ctx().svm)),
            stats.final_user_balances,
        )?;

        if let Some(manager) = self.arbitrage_manager.as_ref() {
            let balances = manager.total_balances(&self.env.ctx().svm);
            let price = self.env.market_price();

            if let Some(current_value_a) = total_value_in_a(balances, price) {
                let profit_a = initial_arb_value_in_a
                    .map(|initial| current_value_a - initial)
                    .unwrap_or(0.0);

                let profit_pct = initial_arb_value_in_a
                    .filter(|initial| *initial > 0.0)
                    .map(|initial| (profit_a / initial) * 100.0);

                tracing::info!(
                    "Final arbitrageur funds: A={} B={} | total_in_A={:.1} | profit_in_A={:.1} | profit_pct={:.4}%",
                    balances.0 / LAMPORTS_PER_SOL,
                    balances.1 / LAMPORTS_PER_SOL,
                    current_value_a / LAMPORTS_PER_SOL as f64,
                    profit_a / LAMPORTS_PER_SOL as f64,
                    profit_pct.unwrap_or(0.0)
                );
            } else {
                tracing::warn!(
                    "Skipping final arbitrageur funds log due to non-positive market price ({})",
                    price
                );
            }
        }

        self.csv_writer.finalize()?;
        Ok(stats)
    }
}

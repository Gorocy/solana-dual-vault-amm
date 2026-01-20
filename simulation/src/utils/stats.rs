use serde::{Deserialize, Serialize};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use tracing::info;

use crate::utils::actors::arbitrageur_bot::ArbitrageStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultReserve {
    pub vault_id: usize,
    pub reserve_a: u64,
    pub reserve_b: u64,
}

impl VaultReserve {
    pub fn new(vault_id: usize, reserve_a: u64, reserve_b: u64) -> Self {
        Self {
            vault_id,
            reserve_a,
            reserve_b,
        }
    }

    pub fn delta(&self, other: &VaultReserve) -> (i64, i64) {
        (
            self.reserve_a as i64 - other.reserve_a as i64,
            self.reserve_b as i64 - other.reserve_b as i64,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationStats {
    pub total_slots: u64,
    pub successful_swaps: u64,
    pub failed_swaps: u64,
    pub initial_reserves: Vec<VaultReserve>,
    pub final_reserves: Vec<VaultReserve>,
    pub arbitrage_profits: Option<ArbitrageStats>,
    pub initial_user_balances: Option<(u64, u64)>,
    pub final_user_balances: Option<(u64, u64)>,
}

impl SimulationStats {
    pub fn new(initial_reserves: Vec<VaultReserve>) -> Self {
        Self {
            total_slots: 0,
            successful_swaps: 0,
            failed_swaps: 0,
            initial_reserves,
            final_reserves: Vec::new(),
            arbitrage_profits: None,
            initial_user_balances: None,
            final_user_balances: None,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.successful_swaps + self.failed_swaps;
        if total > 0 {
            (self.successful_swaps as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn display(&self) {
        info!(" Simulation Results:");
        info!("  Total slots: {}", self.total_slots);
        info!("  Successful swaps: {}", self.successful_swaps);
        info!("  Failed swaps: {}", self.failed_swaps);
        info!("  Success rate: {:.2}%", self.success_rate());

        info!("Vault Reserve Changes:");
        for (final_reserve, initial_reserve) in
            self.final_reserves.iter().zip(self.initial_reserves.iter())
        {
            let (delta_a, delta_b) = final_reserve.delta(initial_reserve);
            info!(
                "  Vault {}: A={}, B={} (Δ: A={:+}, B={:+})(Profit A: {:.5}%, B: {:.5}%)",
                final_reserve.vault_id,
                final_reserve.reserve_a / LAMPORTS_PER_SOL as u64,
                final_reserve.reserve_b / LAMPORTS_PER_SOL as u64,
                delta_a / LAMPORTS_PER_SOL as i64,
                delta_b / LAMPORTS_PER_SOL as i64,
                delta_a as f64 / ( final_reserve.reserve_a as i64 + delta_a ) as f64 * 100.0,
                delta_b as f64 / ( final_reserve .reserve_b as i64 + delta_b ) as f64 * 100.0
            );
        }

        if let Some(arb_stats) = &self.arbitrage_profits {
            info!("  Arbitrage Statistics:");
            info!("  Opportunities found: {}", arb_stats.opportunities_found);
            info!("  Successful: {}", arb_stats.arbitrages_executed);
        }

        info!("User Balance Changes:");
        if let (Some((initial_a, initial_b)), Some((final_a, final_b))) = 
            (self.initial_user_balances, self.final_user_balances) 
        {
            let delta_a = final_a as i64 - initial_a as i64;
            let delta_b = final_b as i64 - initial_b as i64;
            info!(
                "  Total Users: A={}, B={} (Δ: A={:+}, B={:+})(Change A: {:.5}%, B: {:.5}%)",
                final_a / LAMPORTS_PER_SOL as u64,
                final_b / LAMPORTS_PER_SOL as u64,
                delta_a / LAMPORTS_PER_SOL as i64,
                delta_b / LAMPORTS_PER_SOL as i64,
                if initial_a > 0 { delta_a as f64 / initial_a as f64 * 100.0 } else { 0.0 },
                if initial_b > 0 { delta_b as f64 / initial_b as f64 * 100.0 } else { 0.0 }
            );
        } else {
            info!("  User balances not tracked");
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        info!("💾 Stats saved to: {}", path);
        Ok(())
    }
}

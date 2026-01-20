pub mod single;
pub mod dual;
pub mod common;
pub mod manager;
pub(super) mod calc_utils;
pub mod traits;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    /// Minimal profit percentage to execute arbitrage (e.g., 0.01 = 1%)
    pub min_profit_threshold: f64,
    /// Max size of position as % of total balance (e.g., 0.2 = 20%)
    pub max_position_size: f64,
    /// Probability of checking arbitrage in a given slot
    pub check_frequency: f64,
    /// Minimal swap amount
    pub min_swap_amount: u64,
    /// Precision ternary search as % of range (0.001 = 0.1%)
    pub search_precision_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageStats {
    pub opportunities_found: u32,
    pub arbitrages_executed: u32,
    pub failed_arbitrages: u32,
}


#[derive(Debug, Clone)]
pub struct ArbitrageOpportunityGeneric<T> {
    pub strategy_data: T,
    pub is_a_to_b: bool,
    pub amount_in: u64,
    pub expected_output: u64,
    pub market_price: f64,
    pub expected_profit: u64,
    pub profit_percentage: f64,
}

impl ArbitrageStats {
    pub fn new() -> Self {
        Self {
            opportunities_found: 0,
            arbitrages_executed: 0,
            failed_arbitrages: 0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.arbitrages_executed + self.failed_arbitrages;
        if total == 0 {
            0.0
        } else {
            self.arbitrages_executed as f64 / total as f64
        }
    }
}

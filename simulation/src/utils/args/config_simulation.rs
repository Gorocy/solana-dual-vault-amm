use serde::{Deserialize, Serialize};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use tracing::info;

use crate::utils::ix::FeeOption;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub vault_count: u8,
    pub user_count: usize,
    pub balance_a_min: u64,
    pub balance_a_max: u64,
    pub balance_b_min: u64,
    pub balance_b_max: u64,
    pub max_slots: u64,
    pub initial_liquidity_a: u64,
    pub initial_liquidity_b: u64,
    pub swap_frequency: f64,
    pub min_swap_percentage: f64,
    pub max_swap_percentage: f64,
    pub simulation_seed: u64,
    pub output_file: String,
    pub compute_csv_file: String,
    pub arbitrageur_count: u8,
    pub enable_arbitrageurs: bool,
    pub fee_option: FeeOption,
    pub strategy: u8,
    // ArbitrageConfig fields
    pub arb_min_profit_threshold: f64,
    pub arb_max_position_size: f64,
    pub arb_check_frequency: f64,
    pub arb_min_swap_amount: u64,
    pub arb_search_precision_ratio: f64,
    // UserBehaviorConfig fields
    pub user_a_to_b_probability: f64,
    // Market GBM parameters
    pub market_volatility: f64,
    pub market_drift: f64,
}

impl SimulationConfig {
    pub fn log_info(&self) {
        info!("  Simulation Configuration:");
        info!("  Vault count: {}", self.vault_count);
        info!("  User count: {}", self.user_count);
        info!(
            "  User token A balance range: {} - {}",
            self.balance_a_min / LAMPORTS_PER_SOL, self.balance_a_max / LAMPORTS_PER_SOL
        );
        info!(
            "  User token B balance range: {} - {}",
            self.balance_b_min / LAMPORTS_PER_SOL, self.balance_b_max / LAMPORTS_PER_SOL
        );
        info!(
            "  Liqudity token A for each vault: {}",
            self.initial_liquidity_a / LAMPORTS_PER_SOL
        );
        info!(
            "  Liqudity token B for each vault: {}",
            self.initial_liquidity_b / LAMPORTS_PER_SOL
        );
        info!(
            "  Summary of initial liquidity A: {}",
            (self.initial_liquidity_a as u128 * self.vault_count as u128) / LAMPORTS_PER_SOL as u128
        );
        info!(
            "  Summary of initial liquidity B: {}",
            (self.initial_liquidity_b as u128 * self.vault_count as u128) / LAMPORTS_PER_SOL as u128
        );
        info!("  Max slots: {}", self.max_slots);
        info!("  Swap frequency: {:.2}%", self.swap_frequency * 100.0);
        info!("  Output file: {}", self.output_file);
        info!(
            "  Seeds: simulation={}",
            self.simulation_seed
        );
        
        info!("  Market GBM Parameters:");
        info!("  Annual volatility: {:.1}%", self.market_volatility * 100.0);
        info!("  Annual drift: {:.2}% ({})", 
            self.market_drift * 100.0,
            if self.market_drift > 0.02 { "bullish trend" }
            else if self.market_drift < -0.02 { "bearish trend" }
            else { "neutral" }
        );
        info!("  User A->B probability: {:.1}% (correlated with market trend)", 
            self.user_a_to_b_probability * 100.0
        );
        info!("  Time step: 0.4s (Solana block time)");

        if self.enable_arbitrageurs {
            info!("  Arbitrageurs: {}", self.arbitrageur_count);
            info!(
                "  Arbitrage frequency: {:.2}%",
                self.arb_check_frequency * 100.0
            );
            info!(
                "  Min arbitrage profit: {:.2}%",
                self.arb_min_profit_threshold * 100.0
            );
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.vault_count == 0 {
            return Err(ValidationError::InvalidVaultCount);
        }
        if self.strategy == 2 && self.vault_count < 2 {
            return Err(ValidationError::InvalidDualVaultCount);
        }
        if self.user_count == 0 {
            return Err(ValidationError::InvalidUserCount);
        }
        if self.balance_a_min >= self.balance_a_max {
            return Err(ValidationError::InvalidBalanceRange("token A"));
        }
        if self.balance_b_min >= self.balance_b_max {
            return Err(ValidationError::InvalidBalanceRange("token B"));
        }
        if self.swap_frequency < 0.0 || self.swap_frequency > 1.0 {
            return Err(ValidationError::InvalidFrequency("swap"));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid vault count: must be greater than 0")]
    InvalidVaultCount,

    #[error("Invalid vault count for dual strategy: must be at least 2")]
    InvalidDualVaultCount,

    #[error("Invalid user count: must be greater than 0")]
    InvalidUserCount,

    #[error("Invalid balance range for {0}: min must be less than max")]
    InvalidBalanceRange(&'static str),

    #[error("Invalid {0} frequency: must be between 0.0 and 1.0")]
    InvalidFrequency(&'static str),
}

use clap::Parser;

use crate::utils::{
    args::{
        ARBITRAGEUR_COUNT, ARB_CHECK_FREQUENCY, ARB_MAX_POSITION_SIZE,
        ARB_MIN_PROFIT_THRESHOLD, ARB_MIN_SWAP_AMOUNT, ARB_SEARCH_PRECISION_RATIO,
        BALANCE_MAX_A_USER, BALANCE_MAX_B_USER, BALANCE_MIN_A_USER, BALANCE_MIN_B_USER,
        ENABLE_ARBITRAGEURS, FEE_OPTION_BASE, INITIAL_LIQUIDITY_A_VAULT,
        INITIAL_LIQUIDITY_B_VAULT, MAX_SLOTS_SIMULATION, MAX_SWAP_PERCENTAGE,
        MIN_SWAP_PERCENTAGE, SIMULATION_SEED, SWAP_FREQUENCY,
        USERS_FOR_SIMULATION, VAULTS_FOR_SIMULATION,
        MARKET_VOLATILITY,
        config_simulation::SimulationConfig,
    },
    ix::FeeOption,
};

/// Shared CLI arguments for both single- and dual-vault simulations.
#[derive(Parser, Debug, Clone)]
#[command(name = "simulation")]
#[command(about = "Monte Carlo simulation for CPMM trading", long_about = None)]
pub struct SharedArgs {
    /// Number of vaults
    #[arg(long, default_value_t = VAULTS_FOR_SIMULATION)]
    pub vault_count: u8,

    /// Number of simulated users
    #[arg(long, default_value_t = USERS_FOR_SIMULATION)]
    pub user_count: usize,

    /// Minimum token A balance (in lamports)
    #[arg(long, default_value_t = BALANCE_MIN_A_USER)]
    pub balance_a_min: u64,

    /// Maximum token A balance (in lamports)
    #[arg(long, default_value_t = BALANCE_MAX_A_USER)]
    pub balance_a_max: u64,

    /// Minimum token B balance (in lamports)
    #[arg(long, default_value_t = BALANCE_MIN_B_USER)]
    pub balance_b_min: u64,

    /// Maximum token B balance (in lamports)
    #[arg(long, default_value_t = BALANCE_MAX_B_USER)]
    pub balance_b_max: u64,

    /// Maximum number of slots to simulate
    #[arg(long, default_value_t = MAX_SLOTS_SIMULATION)]
    pub max_slots: u64,

    /// Initial liquidity for token A per vault
    #[arg(long, default_value_t = INITIAL_LIQUIDITY_A_VAULT)]
    pub initial_liquidity_a: u64,

    /// Initial liquidity for token B per vault
    #[arg(long, default_value_t = INITIAL_LIQUIDITY_B_VAULT)]
    pub initial_liquidity_b: u64,

    /// Swap frequency (probability per slot)
    #[arg(long, default_value_t = SWAP_FREQUENCY)]
    pub swap_frequency: f64,

    /// Minimum swap percentage
    #[arg(long, default_value_t = MIN_SWAP_PERCENTAGE)]
    pub min_swap_percentage: f64,

    /// Maximum swap percentage
    #[arg(long, default_value_t = MAX_SWAP_PERCENTAGE)]
    pub max_swap_percentage: f64,

    /// Random seed for simulation
    #[arg(long, default_value_t = SIMULATION_SEED)]
    pub simulation_seed: u64,

    /// Output CSV file path
    #[arg(long, default_value = "vault_reserves.csv")]
    pub output_file: String,

    /// Number of arbitrageur bots
    #[arg(long, default_value_t = ARBITRAGEUR_COUNT)]
    pub arbitrageur_count: u8,

    /// Enable arbitrageurs
    #[arg(long, default_value_t = ENABLE_ARBITRAGEURS)]
    pub enable_arbitrageurs: bool,

    /// Fee tier option
    #[arg(long, default_value_t = FEE_OPTION_BASE)]
    pub fee_option: u64,

    /// Arbitrage bot: minimal profit threshold
    #[arg(long, default_value_t = ARB_MIN_PROFIT_THRESHOLD)]
    pub arb_min_profit_threshold: f64,

    /// Arbitrage bot: max position size as fraction of balance
    #[arg(long, default_value_t = ARB_MAX_POSITION_SIZE)]
    pub arb_max_position_size: f64,

    /// Arbitrage bot: check frequency per slot
    #[arg(long, default_value_t = ARB_CHECK_FREQUENCY)]
    pub arb_check_frequency: f64,

    /// Arbitrage bot: minimal swap amount
    #[arg(long, default_value_t = ARB_MIN_SWAP_AMOUNT)]
    pub arb_min_swap_amount: u64,

    /// Arbitrage bot: search precision ratio
    #[arg(long, default_value_t = ARB_SEARCH_PRECISION_RATIO)]
    pub arb_search_precision_ratio: f64,

    /// Compute unit CSV file path for logging gas usage
    #[arg(long, default_value = "vault_reserves_compute.csv")]
    pub compute_csv_file: String,

    /// Market annual volatility (e.g., 0.8 for 80%, typical for crypto)
    #[arg(long, default_value_t = MARKET_VOLATILITY)]
    pub market_volatility: f64,

    /// Disable logging output (for Monte Carlo runs)
    #[arg(long, default_value_t = false)]
    pub quiet: bool,
}

impl SharedArgs {
    pub fn into_config_with_strategy(self, strategy: u8, min_vaults: Option<u8>) -> SimulationConfig {
        let vault_count = min_vaults.map_or(self.vault_count, |min| self.vault_count.max(min));

        // Calculate correlated market drift and user trend from seed
        let market_drift = crate::utils::args::drift_from_seed(
            self.simulation_seed.wrapping_add(crate::utils::args::SEED_OFFSET_MARKET_DRIFT)
        );
        let user_a_to_b_probability = crate::utils::args::user_probability_from_drift(market_drift);

        SimulationConfig {
            vault_count,
            user_count: self.user_count,
            balance_a_min: self.balance_a_min,
            balance_a_max: self.balance_a_max,
            balance_b_min: self.balance_b_min,
            balance_b_max: self.balance_b_max,
            max_slots: self.max_slots,
            initial_liquidity_a: self.initial_liquidity_a,
            initial_liquidity_b: self.initial_liquidity_b,
            swap_frequency: self.swap_frequency,
            min_swap_percentage: self.min_swap_percentage,
            max_swap_percentage: self.max_swap_percentage,
            simulation_seed: self.simulation_seed,
            output_file: self.output_file,
            arbitrageur_count: self.arbitrageur_count,
            enable_arbitrageurs: self.enable_arbitrageurs,
            fee_option: FeeOption::new(self.fee_option),
            strategy,
            compute_csv_file: self.compute_csv_file,
            arb_min_profit_threshold: self.arb_min_profit_threshold,
            arb_max_position_size: self.arb_max_position_size,
            arb_check_frequency: self.arb_check_frequency,
            arb_min_swap_amount: self.arb_min_swap_amount,
            arb_search_precision_ratio: self.arb_search_precision_ratio,
            user_a_to_b_probability,  // Calculated from market drift
            market_volatility: self.market_volatility,
            market_drift,  // Calculated from seed
        }
    }
}

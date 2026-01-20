use solana_sdk::native_token::LAMPORTS_PER_SOL;

pub mod args_shared;
pub mod config_simulation;

pub const USER_MULTIPLIER: u64 = 1_000;

pub const VAULTS_FOR_SIMULATION: u8 = 8;

// Default user count for simulations. Monte Carlo tests will use 32, 128, and 512 users
pub const USERS_FOR_SIMULATION: usize = 128;
pub const BALANCE_MIN_A_USER: u64 = USER_MULTIPLIER * 1 * LAMPORTS_PER_SOL;  // 1 SOL
pub const BALANCE_MAX_A_USER: u64 = USER_MULTIPLIER * 10 * LAMPORTS_PER_SOL; // 10 SOL
pub const BALANCE_MIN_B_USER: u64 = USER_MULTIPLIER * 2 * LAMPORTS_PER_SOL;  // 2 SOL
pub const BALANCE_MAX_B_USER: u64 = USER_MULTIPLIER * 20 * LAMPORTS_PER_SOL; // 20 SOL

pub const MAX_SLOTS_SIMULATION: u64 = 2_000;

pub const INITIAL_LIQUIDITY_A_VAULT: u64 = 100 * USER_MULTIPLIER * LAMPORTS_PER_SOL;
pub const INITIAL_LIQUIDITY_B_VAULT: u64 = 200 * USER_MULTIPLIER * LAMPORTS_PER_SOL;
// Market GBM parameters - configured for cryptocurrency volatility
/// Annual volatility (80% is typical for crypto markets like SOL/USDC)
pub const MARKET_VOLATILITY: f64 = 0.8;
/// Annual drift - calculated from seed via drift_from_seed() in range [-0.15, +0.15]
/// Positive = bullish trend, negative = bearish trend, ~0 = neutral
pub const MARKET_DRIFT: f64 = 0.0;  // Default for configs, overridden by seed in simulation
pub const SWAP_FREQUENCY: f64 = 0.5;
pub const MIN_SWAP_PERCENTAGE: f64 = 0.1;
pub const MAX_SWAP_PERCENTAGE: f64 = 0.3;
pub const SIMULATION_SEED: u64 = 42;
pub const ARBITRAGEUR_COUNT: u8 = VAULTS_FOR_SIMULATION;
pub const ENABLE_ARBITRAGEURS: bool = false;
pub const FEE_OPTION_BASE: u64 = 30;

pub const DEFAULT_OUTPUT_FILE: &str = "vault_reserves.csv";
pub const DEFAULT_COMPUTE_CSV_FILE: &str = "vault_reserves_compute.csv";
pub const SINGLE_OUTPUT_FILE: &str = "single_vault_reserves.csv";
pub const SINGLE_COMPUTE_CSV_FILE: &str = "single_vault_reserves_single_swap_compute.csv";
pub const DUAL_OUTPUT_FILE: &str = "dual_vault_reserves.csv";
pub const DUAL_COMPUTE_CSV_FILE: &str = "dual_vault_reserves_dual_swap_compute.csv";

// ArbitrageConfig constants
pub const ARB_MIN_PROFIT_THRESHOLD: f64 = 0.0;          // 0.0%
pub const ARB_MAX_POSITION_SIZE: f64 = 0.4;             // 40%
pub const ARB_CHECK_FREQUENCY: f64 = 0.90;              // 90% chance per slot
pub const ARB_MIN_SWAP_AMOUNT: u64 = 2000;              // 2000 tokens minimum
pub const ARB_SEARCH_PRECISION_RATIO: f64 = 0.0001;     // 0.001%

// UserBehaviorConfig constants
pub const USER_MIN_SWAP_AMOUNT_PERCENTAGE: f64 = 0.10;  // 10%
pub const USER_MAX_SWAP_AMOUNT_PERCENTAGE: f64 = 0.25;  // 25%
/// User swap frequency set to 50% - intentionally high to create significant market noise
/// This generates high trading volume to stress-test the rebalancing mechanism
/// Tested with 32, 128, and 512 users in Monte Carlo simulations
pub const USER_SWAP_FREQUENCY: f64 = 0.5;               // 50%
/// User A->B probability - calculated from market drift via user_probability_from_drift()
/// Correlated with market trend: bullish drift -> more A->B, bearish -> more B->A
pub const USER_A_TO_B_PROBABILITY: f64 = 0.5;           // Default for configs, overridden by trend in simulation

// Seed offsets for different components to ensure non-overlapping random sequences
// These large prime numbers help avoid seed collisions between different RNG streams
pub const SEED_OFFSET_USER_BATCH: u64 = 2077;           // User creation
pub const SEED_OFFSET_USER_MANAGER: u64 = 3001;         // User manager RNG
pub const SEED_OFFSET_ARBITRAGEUR_BASE: u64 = 5003;     // Arbitrageur bot base
pub const SEED_OFFSET_MARKET_PRICE: u64 = 7919;         // Market price generator
pub const SEED_OFFSET_MARKET_DRIFT: u64 = 11003;        // Market drift/trend
pub const SEED_OFFSET_USER_TREND: u64 = 13007;          // User behavior trend

/// Derive a drift/trend value from seed in range [-0.15, +0.15]
/// This creates correlated market and user trends based on the simulation seed
/// Positive drift = bullish trend, negative = bearish trend
pub fn drift_from_seed(seed: u64) -> f64 {
    // Use the seed to generate a deterministic value in [0, 1]
    let normalized = ((seed % 10000) as f64) / 10000.0;
    // Map to [-0.15, +0.15] range for annual drift
    (normalized * 0.3) - 0.15
}

/// Derive user A->B probability from market trend
/// When market drift is positive (price rising), users prefer buying B (A->B)
/// When market drift is negative (price falling), users prefer buying A (B->A)
pub fn user_probability_from_drift(drift: f64) -> f64 {
    // drift in [-0.15, +0.15] -> probability in [0.4, 0.6]
    // Positive drift increases A->B probability (buy B when price rising)
    // Negative drift decreases A->B probability (buy A when price falling)
    0.5 + (drift / 0.15) * 0.1
}

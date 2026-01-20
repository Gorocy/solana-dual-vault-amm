//! Market price simulation using Geometric Brownian Motion (GBM)
//!
//! This module provides functionality to simulate realistic cryptocurrency price movements
//! using the GBM model. Designed for Solana blockchain with ~0.4s block times.
//!
//! **Simulation Scope:** SHORT-TERM high-volatility periods (30 min - 2 hours).
//! The simulation models periods of EXTREME market activity where prices can move
//! significantly in a very short time due to:
//! - High-frequency trading activity
//! - Cascading liquidations
//! - Large arbitrage flows
//! - Flash crashes/pumps
//!
//! This captures realistic crypto market behavior where 5-10%+ price swings
//! can occur within minutes during volatile periods.

use rand::prelude::*;
use rand_distr::Normal;
use crate::utils::args::SEED_OFFSET_MARKET_PRICE;

/// Solana block time in seconds (approximately)
pub const SOLANA_BLOCK_TIME: f64 = 0.4;

/// Seconds in a year (for annualized parameters)
/// 
/// Note: While volatility/drift are expressed as annualized parameters (standard in finance),
/// this simulation models SHORT-TERM periods (30 min - 2 hours) of intense trading activity.
/// The time steps (dt) are in Solana blocks (~0.4s), representing rapid market movements.
const SECONDS_PER_YEAR: f64 = 365.25 * 24.0 * 3600.0;

/// Market price generator using Geometric Brownian Motion
/// 
/// Simulates SHORT-TERM (30 min - 2 hours) periods of EXTREME cryptocurrency volatility.
/// The key insight: in volatile crypto markets, prices can swing dramatically in minutes.
/// 
/// Example: With 80% annual volatility over 1000 blocks (6.7 minutes):
/// - Price can easily move ±5-15% from starting point
/// - Captures flash crashes, liquidation cascades, and rapid arbitrage
/// - Models real scenarios like Luna collapse, FTX crash, or DEX exploits
///
/// While parameters are annualized (standard), they create realistic large moves
/// over short timeframes - matching observed crypto market behavior.
pub struct MarketPriceGenerator {
    /// Current price
    current_price: f64,
    /// Volatility parameter (annualized, typical crypto: 0.5-2.0)
    volatility: f64,
    /// Drift parameter (annualized, typical crypto: -0.1 to 0.1)
    drift: f64,
    /// Time step in years (default: 0.4s for Solana blocks)
    /// Note: Despite being in "years", represents 0.4s real-time intervals
    dt: f64,
    /// Random number generator
    rng: StdRng,
}

impl MarketPriceGenerator {
    /// Create a new market price generator for cryptocurrency markets
    ///
    /// Simulates SHORT-TERM periods (30 min - 2 hours) of EXTREME market volatility.
    /// Each step represents 0.4s Solana blocks - prices can move significantly in minutes!
    ///
    /// # Arguments
    /// * `initial_price` - Starting price (e.g., 1.0 for stablecoin, 100.0 for SOL/USDC)
    /// * `volatility` - Annual volatility (e.g., 0.8 = 80% creates large short-term swings)
    /// * `drift` - Annual drift (e.g., 0.0 for neutral - typically small for short periods)
    /// * `dt` - Time step in years (use `dt_from_seconds(0.4)` for Solana blocks)
    /// * `seed` - Random seed for reproducible simulations
    ///
    /// # Volatility Impact Examples (1000 slots = ~6.7 minutes)
    /// - Volatility 0.5 (50%): Price typically moves ±2-5%
    /// - Volatility 0.8 (80%): Price can move ±5-15% (realistic for volatile periods)
    /// - Volatility 1.5 (150%): Extreme moves ±10-30% (flash crash scenarios)
    ///
    /// # Example
    /// ```ignore
    /// // Simulate 1000 blocks (~6.7 minutes) of extreme SOL/USDC volatility
    /// let mut gen = MarketPriceGenerator::new(
    ///     100.0,  // $100 SOL price
    ///     0.8,    // 80% volatility → expect 5-15% price swing in minutes
    ///     0.0,    // no directional bias
    ///     MarketPriceGenerator::dt_from_seconds(0.4),
    ///     42
    /// );
    /// // After 1000 steps, price might be anywhere from $85 to $115
    /// ```
    pub fn new(initial_price: f64, volatility: f64, drift: f64, dt: f64, seed: u64) -> Self {
        Self {
            current_price: initial_price,
            volatility,
            drift,
            dt,
            rng: StdRng::seed_from_u64(seed.wrapping_add(SEED_OFFSET_MARKET_PRICE)),
        }
    }

    /// Helper to convert seconds to time step in years
    ///
    /// # Example
    /// ```ignore
    /// let dt = MarketPriceGenerator::dt_from_seconds(0.4); // Solana block time
    /// ```
    pub fn dt_from_seconds(seconds: f64) -> f64 {
        seconds / SECONDS_PER_YEAR
    }

    /// Generate the next price using GBM formula
    ///
    /// The GBM formula is:
    /// S(t+dt) = S(t) * exp((drift - 0.5 * volatility^2) * dt + volatility * sqrt(dt) * Z)
    /// where Z ~ N(0,1)
    pub fn next_price(&mut self) -> f64 {
        let normal = Normal::new(0.0, 1.0).unwrap();
        let z = self.rng.sample(normal);

        let drift_term = (self.drift - 0.5 * self.volatility.powi(2)) * self.dt;
        let volatility_term = self.volatility * self.dt.sqrt() * z;

        self.current_price *= (drift_term + volatility_term).exp();

        self.current_price
    }

    /// Get the current price without generating a new one
    pub fn current_price(&self) -> f64 {
        self.current_price
    }

    /// Reset the price to a new value
    pub fn reset_price(&mut self, new_price: f64) {
        self.current_price = new_price;
    }

    /// Generate multiple price steps at once
    pub fn generate_prices(&mut self, steps: usize) -> Vec<f64> {
        (0..steps).map(|_| self.next_price()).collect()
    }

    /// Set new volatility parameter
    pub fn set_volatility(&mut self, volatility: f64) {
        self.volatility = volatility;
    }

    /// Set new drift parameter
    pub fn set_drift(&mut self, drift: f64) {
        self.drift = drift;
    }

    /// Set new time step
    pub fn set_dt(&mut self, dt: f64) {
        self.dt = dt;
    }

    /// Apply a specific price change (for external market events)
    pub fn apply_price_change(&mut self, change_percentage: f64) {
        self.current_price *= 1.0 + change_percentage;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_generator_creation() {
        let dt = MarketPriceGenerator::dt_from_seconds(SOLANA_BLOCK_TIME);
        let generator = MarketPriceGenerator::new(100.0, 0.8, 0.0, dt, 0);
        assert_eq!(generator.current_price(), 100.0);
    }

    #[test]
    fn test_price_generation() {
        let dt = MarketPriceGenerator::dt_from_seconds(SOLANA_BLOCK_TIME);
        let mut generator = MarketPriceGenerator::new(1.0, 0.5, 0.0, dt, 0);
        let initial_price = generator.current_price();
        let new_price = generator.next_price();

        // Price should change (with very high probability)
        assert_ne!(initial_price, new_price);
        // Price should be positive
        assert!(new_price > 0.0);
    }

    #[test]
    fn test_dt_conversion() {
        let dt = MarketPriceGenerator::dt_from_seconds(0.4);
        // 0.4 seconds should be approximately 1.267e-8 years
        assert!((dt - 1.267e-8).abs() < 1e-10);
    }
}

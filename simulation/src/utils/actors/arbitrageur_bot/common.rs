use rand::{rngs::StdRng, Rng, SeedableRng};
use tracing::trace;

use crate::utils::actors::{account::SimUser, arbitrageur_bot::ArbitrageConfig};

const SAFETY_MAX_ITERATIONS: usize = 128 * 2;

#[derive(Debug, Clone)]
pub struct ArbitrageCandidate {
    pub amount_in: u64,
    pub expected_output: u64,
    pub net_profit: f64,
    pub profit_percentage: f64,
}

pub struct CommonArbitrageBot {
    pub sim_user: SimUser,
    pub config: ArbitrageConfig,
    rng: StdRng,
}

impl CommonArbitrageBot {
    pub fn new(sim_user: SimUser, config: ArbitrageConfig, seed: u64) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        Self {
            sim_user,
            config,
            rng,
        }
    }

    pub fn should_check(&mut self) -> bool {
        self.rng.random::<f64>() < self.config.check_frequency
    }

    fn ternary_search_optimal_amount<F>(
        &self,
        balance: u64,
        evaluate_profit: F,
    ) -> Option<(u64, u64, f64)>
    where
        F: Fn(u64) -> Option<(u64, f64)>,
    {
        let max_amount = (balance as f64 * self.config.max_position_size) as u64;
        let min_amount = self.config.min_swap_amount;

        if max_amount <= min_amount || balance == 0 {
            return None;
        }

        let mut left = min_amount;
        let mut right = max_amount;

        // STEP 1: Find valid upper bound
        trace!("Finding valid upper bound...");
        let profit_left = evaluate_profit(left);

        if profit_left.is_none() {
            trace!("Even minimum amount {} doesn't work", left);
            return None;
        }

        // Search for the upper bound using doubling method, starting from small values
        let mut test_amount = left * 2;
        let mut last_valid = left;
        let mut last_valid_result = profit_left;

        while test_amount <= right {
            if let Some(result) = evaluate_profit(test_amount) {
                last_valid = test_amount;
                last_valid_result = Some(result);
                trace!("Amount {} works, profit={:.2}", test_amount, result.1);
                test_amount *= 2; // Doubling
            } else {
                trace!(
                    "Amount {} doesn't work, upper bound is between {} and {}",
                    test_amount,
                    last_valid,
                    test_amount
                );
                // Found the upper bound
                right = test_amount;
                break;
            }
        }

        // If even max_amount works, use it as right
        if test_amount > right && last_valid == test_amount / 2 {
            if let Some(result) = evaluate_profit(right) {
                last_valid = right;
                last_valid_result = Some(result);
            } else {
                right = last_valid * 2; // Safe upper bound
            }
        }

        trace!("Valid range found: {} to {}", left, right);

        // STEP 2: Binary refinement of the upper bound (find the exact boundary)
        if right > last_valid * 2 {
            let mut binary_left = last_valid;
            let mut binary_right = right;

            while binary_right - binary_left > binary_left / 10 {
                let mid = (binary_left + binary_right) / 2;
                if evaluate_profit(mid).is_some() {
                    binary_left = mid;
                    last_valid = mid;
                } else {
                    binary_right = mid;
                }
            }
            right = last_valid;
            trace!("Refined upper bound: {}", right);
        }

        // STEP 3: Ternary search in the found range
        let mut iterations = 0;
        let mut best_amount = left;
        let mut best_profit = last_valid_result.unwrap().1;

        while right - left > ((left as f64 * self.config.search_precision_ratio) as u64).max(1) {
            iterations += 1;
            if iterations > SAFETY_MAX_ITERATIONS {
                break;
            }

            let prev_left = left;
            let prev_right = right;
            let mid1 = left + (right - left) / 3;
            let mid2 = left + 2 * (right - left) / 3;

            let profit1 = evaluate_profit(mid1);
            let profit2 = evaluate_profit(mid2);

            trace!(
                "Iteration {}: mid1={} profit={:?}, mid2={} profit={:?}",
                iterations,
                mid1,
                profit1,
                mid2,
                profit2
            );

            match (profit1, profit2) {
                (Some((_, p1)), Some((_, p2))) => {
                    // Tracking the best result
                    if p1 > best_profit {
                        best_profit = p1;
                        best_amount = mid1;
                    }
                    if p2 > best_profit {
                        best_profit = p2;
                        best_amount = mid2;
                    }

                    if p1 < p2 {
                        left = mid1;
                    } else {
                        right = mid2;
                    }
                }
                (Some((_, p1)), None) => {
                    if p1 > best_profit {
                        best_profit = p1;
                        best_amount = mid1;
                    }
                    right = mid2;
                }
                (None, Some((_, p2))) => {
                    if p2 > best_profit {
                        best_profit = p2;
                        best_amount = mid2;
                    }
                    left = mid1;
                }
                (None, None) => {
                    // Both points are invalid - use the last known good one
                    break;
                }
            }

            if left == prev_left && right == prev_right {
                break;
            }
        }

        // STEP 4: Finalization - check the middle and edges
        let final_amount = (left + right) / 2;
        let candidates = [best_amount, left, right, final_amount];

        let mut best_result = None;
        let mut best_final_profit = f64::MIN;

        for &candidate in &candidates {
            if let Some((output, profit)) = evaluate_profit(candidate) {
                if profit > best_final_profit {
                    best_final_profit = profit;
                    best_result = Some((candidate, output, profit));
                }
            }
        }

        if let Some(result) = best_result {
            trace!(
                "Final result: amount={}, output={}, profit={:.2}",
                result.0,
                result.1,
                result.2
            );
            Some(result)
        } else {
            trace!("No valid result found");
            None
        }
    }

    pub fn find_best_with_profit<F>(
        &self,
        balance: u64,
        evaluate_profit: F,
        amount_in_value_a: impl Fn(u64) -> f64,
    ) -> Option<ArbitrageCandidate>
    where
        F: Fn(u64) -> Option<(u64, f64)>,
    {
        let (amount_in, expected_output, net_profit) =
            self.ternary_search_optimal_amount(balance, evaluate_profit)?;

        let profit_percentage = net_profit / amount_in_value_a(amount_in) * 100.0;

        Some(ArbitrageCandidate {
            amount_in,
            expected_output,
            net_profit,
            profit_percentage,
        })
    }
}

use solana_sdk::{pubkey::Pubkey, signer::Signer};
use tracing::{debug, trace};

use crate::{
    error::ResultSimulation,
    utils::{
        actors::{
            SwapStrategy, TradingRoundStats, account::SimUser, user::{UserSwapStrategy, dual::DualVaultStrategy, single::SingleVaultStrategy, user_batch::UserStats},
        },
        ix::context::SimContext,
    },
};
use super::utils::ManagerUtils;
use rand::{rngs::StdRng, Rng, SeedableRng};

pub use crate::utils::actors::user::behavior::{UserBehaviorConfig, UserBehaviorConfigBuilder};

pub struct UserManager {
    pub users: Vec<SimUser>,
    pub behavior_config: UserBehaviorConfig,
    rng: StdRng,
    pub vaults: Vec<(u8, u8)>,
    strategy_impl: Box<dyn UserSwapStrategy>,
}

impl UserManager {
    pub fn new(users: Vec<SimUser>, behavior_config: UserBehaviorConfig, seed: u64) -> Self {
        let strategy_impl: Box<dyn UserSwapStrategy> = match behavior_config.strategy {
            SwapStrategy::SingleVault => Box::new(SingleVaultStrategy),
            SwapStrategy::DualVault => Box::new(DualVaultStrategy),
        };
        Self {
            users,
            behavior_config,
            rng: StdRng::seed_from_u64(seed),
            vaults: Vec::new(),
            strategy_impl,
        }
    }

    pub fn execute_random_trading_round_with_compute(
        &mut self,
        ctx: &mut SimContext,
        registry_pubkey: Pubkey,
        vault_count: u8,
        slot: u64,
        mut compute_writer: Option<&mut crate::output::SwapComputeCsvWriter>,
    ) -> ResultSimulation<TradingRoundStats> {
        let mut stats = TradingRoundStats::default();

        self.strategy_impl.ensure_vaults_available(vault_count, &self.vaults)?;
        trace!("Try execute random");

        for idx in 0..self.users.len() {
            let user = &self.users[idx];
            if !Self::should_user_trade(&mut self.rng, &self.behavior_config) {
                continue;
            }

            let Some((is_a_to_b, available_balance)) =
                ManagerUtils::pick_direction_and_balance(&mut self.rng, &self.behavior_config, user, ctx)
            else {
                continue;
            };

            let Some(amount_in) =
                ManagerUtils::sample_amount(&mut self.rng, &self.behavior_config, available_balance)
            else {
                continue;
            };

            let Some(vault_index) = self
                .strategy_impl
                .pick_vault_index(&mut self.rng, vault_count, &self.vaults)
            else {
                continue;
            };

            match self.strategy_impl.execute_user_swap(
                ctx,
                user,
                registry_pubkey,
                amount_in,
                is_a_to_b,
                vault_index,
                &self.vaults,
            ) {
                Ok(swap_result) => {
                    ManagerUtils::record_swap_stats(&mut stats, &swap_result);
                    if let Some(w) = compute_writer.as_mut() {
                        ManagerUtils::write_swap_compute(w, slot, &swap_result)?;
                    }
                }
                Err(_) => stats.failed_swaps += 1,
            }
        }

        debug!(
            "Trading round completed: {} successful, {} failed swaps",
            stats.successful_swaps, stats.failed_swaps
        );

        Ok(stats)
    }

    fn should_user_trade(rng: &mut StdRng, config: &UserBehaviorConfig) -> bool {
        rng.random::<f64>() < config.swap_frequency
    }

    /// Zwraca statystyki użytkowników (balanse z SVM)
    pub fn get_user_stats(&self, ctx: &SimContext) -> Vec<UserStats> {
        self.users
            .iter()
            .enumerate()
            .map(|(i, user)| UserStats {
                user_index: i,
                pubkey: user.keypair.pubkey(),
                balance_a: user.get_balance_a(&ctx.svm),
                balance_b: user.get_balance_b(&ctx.svm),
            })
            .collect()
    }

    pub fn total_user_balances(&self, ctx: &SimContext) -> (u64, u64) {
        self.users.iter().fold((0u64, 0u64), |(sum_a, sum_b), user| {
            let (a, b) = user.get_balances(&ctx.svm);
            (sum_a.saturating_add(a), sum_b.saturating_add(b))
        })
    }

    /// Reset lamport balances for all users to prevent running out of SOL for transaction fees
    /// Called periodically (every 100 slots) to ensure users can continue trading
    pub fn reset_lamport_balances(&self, ctx: &mut SimContext, target_lamports: u64) -> ResultSimulation<()> {
        for user in &self.users {
            let user_pubkey = user.keypair.pubkey();
            
            // Simply set account lamports to target amount
            if let Some(mut account) = ctx.svm.get_account(&user_pubkey) {
                account.lamports = target_lamports;
                ctx.svm.set_account(user_pubkey, account).map_err(|e| {
                    crate::error::SimulationError::SystemError(format!("Failed to reset lamports for user {}: {:?}", user_pubkey, e))
                })?;
            } else {
                // If account doesn't exist, airdrop
                ctx.airdrop(&user_pubkey, target_lamports)?;
            }
        }
        Ok(())
    }
}
use cpmm_rebalansing::{utils::math::swap_dual::SwapUtilsVirtualVault, VaultAssets};
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, trace};

use crate::{
    error::{ResultSimulation, SimulationError},
    utils::{
        actors::{
            account::SimUser,
            arbitrageur_bot::{
                ArbitrageConfig, dual::{DualArbitrageOpportunity, DualVaultData}, manager::ManagedArbitrageBot, traits::{ArbitrageBotFactory, ArbitrageStrategy, GenericArbitrageBot}
            },
        },
        enviroment::setup::MarketSimulationEnvironment,
        ix::{
            FeeOption, context::SimContext, dual_swap_utils::DualSwapUtils, vault_utils::VaultUtils
        },
    },
};

pub struct DualArbitrageBot {
    inner: GenericArbitrageBot<DualVaultStrategy>,
}

impl DualArbitrageBot {
    pub fn new(
        ctx: &mut SimContext,
        mint_a: Pubkey,
        mint_b: Pubkey,
        initial_balance_a: u64,
        initial_balance_b: u64,
        config: ArbitrageConfig,
        seed: u64,
    ) -> ResultSimulation<Self> {
        Ok(Self {
            inner: GenericArbitrageBot::new(
                ctx,
                mint_a,
                mint_b,
                initial_balance_a,
                initial_balance_b,
                config,
                seed,
                DualVaultStrategy,
            )?,
        })
    }

    pub fn sim_user(&self) -> &SimUser {
        self.inner.sim_user()
    }

    pub fn find_opportunity(
        &mut self,
        env: &mut MarketSimulationEnvironment,
        current_slot: u64,
        vaults: &[(u8, u8)],
    ) -> ResultSimulation<Option<DualArbitrageOpportunity>> {
        if vaults.is_empty() {
            return Ok(None);
        }

        let market_price = env.market.current_price();
        debug!(
            slot = current_slot,
            market_price, "Checking dual vault arbitrage (filtered pairs)"
        );

        let (balance_a, balance_b) = self.inner.common.sim_user.get_balances(&env.ctx.svm);
        let fee_rate = env.fee_rate.clone();
        let mut best: Option<DualArbitrageOpportunity> = None;

        for &(i, j) in vaults {
            if i >= env.vaults.len() as u8 || j >= env.vaults.len() as u8 || i == j {
                continue;
            }

            let strategy_data = DualVaultData {
                vault1_index: i,
                vault2_index: j,
            };

            if let Some(opportunity) = self.inner.find_opportunity_for_target(
                env,
                strategy_data,
                market_price,
                balance_a,
                balance_b,
                &fee_rate,
            )? {
                let is_better = best
                    .as_ref()
                    .map(|b| opportunity.expected_profit > b.expected_profit)
                    .unwrap_or(true);

                if is_better {
                    debug!(vault1 = i, vault2 = j, "New best dual opportunity");
                    best = Some(opportunity);
                }
            }
        }

        Ok(best)
    }

    pub fn check_and_execute(
        &mut self,
        env: &mut MarketSimulationEnvironment,
        current_slot: u64,
        vaults: &[(u8, u8)],
    ) -> ResultSimulation<Option<DualArbitrageOpportunity>> {
        if !self.inner.common.should_check() {
            return Ok(None);
        }

        if let Some(opportunity) = self.find_opportunity(env, current_slot, vaults)? {
            let actual_output = self.inner.execute_opportunity(env, &opportunity)?;

            trace!(
                vault1_index = opportunity.strategy_data.vault1_index,
                vault2_index = opportunity.strategy_data.vault2_index,
                amount_in = opportunity.amount_in,
                actual_output,
                "Dual arbitrage executed"
            );

            Ok(Some(opportunity))
        } else {
            Ok(None)
        }
    }
}

impl ManagedArbitrageBot for DualArbitrageBot {
    type Environment = MarketSimulationEnvironment;
    type Opportunity = DualArbitrageOpportunity;
    type SharedState = Vec<(u8, u8)>;

    fn check_and_execute(
        &mut self,
        env: &mut Self::Environment,
        current_slot: u64,
        shared: &Self::SharedState,
    ) -> ResultSimulation<Option<Self::Opportunity>> {
        self.check_and_execute(env, current_slot, shared)
    }

    fn sim_user(&self) -> &SimUser {
        self.sim_user()
    }

    fn on_success(shared: &mut Self::SharedState, opportunity: &Self::Opportunity) {
        let pair = (
            opportunity.strategy_data.vault1_index,
            opportunity.strategy_data.vault2_index,
        );

        if !shared.contains(&pair) {
            shared.push(pair);
        }
    }
}

impl ArbitrageBotFactory for DualArbitrageBot {
    fn create(
        ctx: &mut SimContext,
        mint_a: Pubkey,
        mint_b: Pubkey,
        initial_balance_a: u64,
        initial_balance_b: u64,
        config: ArbitrageConfig,
        seed: u64,
    ) -> ResultSimulation<Self> {
        Self::new(
            ctx,
            mint_a,
            mint_b,
            initial_balance_a,
            initial_balance_b,
            config,
            seed,
        )
    }
}

pub(super) struct DualVaultStrategy;

impl ArbitrageStrategy for DualVaultStrategy {
    type StrategyData = DualVaultData;
    type Environment = MarketSimulationEnvironment;
    type ReserveType = (VaultAssets, VaultAssets);

    fn get_reserves(
        &self,
        env: &mut Self::Environment,
        strategy_data: &Self::StrategyData,
    ) -> ResultSimulation<(VaultAssets, VaultAssets)> {
        let vault1_pda = env.vaults[strategy_data.vault1_index as usize];
        let vault2_pda = env.vaults[strategy_data.vault2_index as usize];

        let (vault1_reserve_a, vault1_reserve_b) =
            VaultUtils::get_vault_reserves(&mut env.ctx, vault1_pda, env.mint_a, env.mint_b)?;

        let (vault2_reserve_a, vault2_reserve_b) =
            VaultUtils::get_vault_reserves(&mut env.ctx, vault2_pda, env.mint_a, env.mint_b)?;

        Ok((
            VaultAssets {
                incoming: vault1_reserve_a as u128,
                outgoing: vault1_reserve_b as u128,
            },
            VaultAssets {
                incoming: vault2_reserve_a as u128,
                outgoing: vault2_reserve_b as u128,
            },
        ))
    }

    fn swap_reserves(&self, reserves: &Self::ReserveType) -> Self::ReserveType {
        (
            VaultAssets {
                incoming: reserves.0.outgoing,
                outgoing: reserves.0.incoming,
            },
            VaultAssets {
                incoming: reserves.1.outgoing,
                outgoing: reserves.1.incoming,
            },
        )
    }

    fn calculate_output(
        &self,
        amount_in: u64,
        reserves: &Self::ReserveType,
        fee_rate: &FeeOption,
    ) -> ResultSimulation<u64> {
        match SwapUtilsVirtualVault::calculate_routing_and_output(
            amount_in,
            fee_rate.normal(),
            &reserves.0,
            &reserves.1,
        ) {
            Ok((total_output, _, _, _)) => Ok(total_output),
            Err(e) => Err(SimulationError::SystemError(format!(
                "Dual swap calculation failed: {:?}",
                e
            ))),
        }
    }

    fn execute_swap(
        &self,
        env: &mut Self::Environment,
        sim_user: &SimUser,
        strategy_data: &Self::StrategyData,
        amount_in: u64,
        is_a_to_b: bool,
    ) -> ResultSimulation<u64> {
        DualSwapUtils::execute_dual_swap(
            &mut env.ctx,
            Some(&sim_user.keypair),
            env.registry,
            strategy_data.vault1_index,
            strategy_data.vault2_index,
            amount_in,
            1,
            is_a_to_b,
        )
    }

    fn rebalance_user(
        &self,
        sim_user: &SimUser,
        env: &mut Self::Environment,
        market_price: f64,
    ) -> ResultSimulation<()> {
        sim_user.rebalance_to_5050(&mut env.ctx, market_price).map(|_| ())
    }
}

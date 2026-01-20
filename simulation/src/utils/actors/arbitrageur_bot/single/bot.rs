use solana_sdk::pubkey::Pubkey;
use tracing::{debug, trace};

use crate::{
    error::{ResultSimulation, SimulationError},
    utils::{
        actors::{
            account::SimUser,
            arbitrageur_bot::{
                manager::ManagedArbitrageBot,
                ArbitrageConfig,
                single::{SingleArbitrageOpportunity, SingleVaultData},
                traits::{ArbitrageBotFactory, ArbitrageStrategy, GenericArbitrageBot}
            },
        },
        enviroment::setup::MarketSimulationEnvironment,
        ix::{FeeOption, context::SimContext, single_swap_utils::SingleSwapUtils, vault_utils::VaultUtils},
    },
};
use cpmm_rebalansing::SwapUtilsSingle;

pub struct SingleArbitrageBot {
    inner: GenericArbitrageBot<SingleVaultStrategy>,
}

impl SingleArbitrageBot {
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
                SingleVaultStrategy,
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
    ) -> ResultSimulation<Option<SingleArbitrageOpportunity>> {
        let market_price = env.market.current_price();
        debug!(
            slot = current_slot,
            market_price, "Checking single vault arbitrage"
        );

        let (balance_a, balance_b) = self.inner.common.sim_user.get_balances(&env.ctx.svm);
        let fee_rate = env.fee_rate.clone();
        let mut best: Option<SingleArbitrageOpportunity> = None;

        for vault_index in 0..env.vaults.len() {
            let strategy_data = SingleVaultData {
                vault_index: vault_index as u8,
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
                    debug!(vault = vault_index, "New best single opportunity");
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
    ) -> ResultSimulation<Option<SingleArbitrageOpportunity>> {
        if !self.inner.common.should_check() {
            return Ok(None);
        }

        if let Some(opportunity) = self.find_opportunity(env, current_slot)? {
            let actual_output = self.inner.execute_opportunity(env, &opportunity)?;

            trace!(
                vault_index = opportunity.strategy_data.vault_index,
                amount_in = opportunity.amount_in,
                actual_output,
                "Single arbitrage executed"
            );

            Ok(Some(opportunity))
        } else {
            Ok(None)
        }
    }
}

impl ManagedArbitrageBot for SingleArbitrageBot {
    type Environment = MarketSimulationEnvironment;
    type Opportunity = SingleArbitrageOpportunity;
    type SharedState = ();

    fn check_and_execute(
        &mut self,
        env: &mut Self::Environment,
        current_slot: u64,
        _shared: &Self::SharedState,
    ) -> ResultSimulation<Option<Self::Opportunity>> {
        self.check_and_execute(env, current_slot)
    }

    fn sim_user(&self) -> &SimUser {
        self.sim_user()
    }
}

impl ArbitrageBotFactory for SingleArbitrageBot {
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

pub struct SingleVaultStrategy;

impl ArbitrageStrategy for SingleVaultStrategy {
    type StrategyData = SingleVaultData;
    type Environment = MarketSimulationEnvironment;
    type ReserveType = (u64, u64);

    fn get_reserves(
        &self,
        env: &mut Self::Environment,
        strategy_data: &Self::StrategyData,
    ) -> ResultSimulation<(u64, u64)> {
        let vault_pda = env.vaults[strategy_data.vault_index as usize];
        VaultUtils::get_vault_reserves(&mut env.ctx, vault_pda, env.mint_a, env.mint_b)
    }

    fn swap_reserves(
        &self,
        reserves: &Self::ReserveType,
    ) -> Self::ReserveType {
        (reserves.1, reserves.0)
    }

    fn calculate_output(
        &self,
        amount_in: u64,
        reserves: &Self::ReserveType,
        fee_rate: &FeeOption,
    ) -> ResultSimulation<u64> {
        match SwapUtilsSingle::calculate_output(amount_in, reserves.0, reserves.1, fee_rate.normal()) {
            Ok((amount_out, _)) => Ok(amount_out),
            Err(e) => Err(SimulationError::SystemError(format!(
                "Single swap calculation failed: {:?}",
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
        SingleSwapUtils::execute_single_swap(
            &mut env.ctx,
            Some(&sim_user.keypair),
            env.registry,
            strategy_data.vault_index,
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

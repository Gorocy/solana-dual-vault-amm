use solana_sdk::pubkey::Pubkey;

use crate::{
    error::ResultSimulation,
    utils::{
        actors::{
            account::SimUser,
            arbitrageur_bot::{
                calc_utils::ProfitCalculator, common::CommonArbitrageBot, ArbitrageConfig,
                ArbitrageOpportunityGeneric,
            },
        },
        ix::{context::SimContext, FeeOption},
    },
};

/// Factory for constructing concrete arbitrage bots in a generic way.
pub trait ArbitrageBotFactory: Sized {
    fn create(
        ctx: &mut SimContext,
        mint_a: Pubkey,
        mint_b: Pubkey,
        initial_balance_a: u64,
        initial_balance_b: u64,
        config: ArbitrageConfig,
        seed: u64,
    ) -> ResultSimulation<Self>;
}

pub trait ArbitrageStrategy {
    type StrategyData: Clone;
    type Environment;
    type ReserveType;

    fn get_reserves(
        &self,
        env: &mut Self::Environment,
        strategy_data: &Self::StrategyData,
    ) -> ResultSimulation<Self::ReserveType>;

    fn swap_reserves(&self, reserves: &Self::ReserveType) -> Self::ReserveType;

    fn calculate_output(
        &self,
        amount_in: u64,
        reserves: &Self::ReserveType,
        fee_rate: &FeeOption,
    ) -> ResultSimulation<u64>;

    fn execute_swap(
        &self,
        env: &mut Self::Environment,
        sim_user: &SimUser,
        strategy_data: &Self::StrategyData,
        amount_in: u64,
        is_a_to_b: bool,
    ) -> ResultSimulation<u64>;

    /// Rebalance user's portfolio to 50/50 after arbitrage
    /// This simulates trading on external market at market_price
    fn rebalance_user(
        &self,
        sim_user: &SimUser,
        env: &mut Self::Environment,
        market_price: f64,
    ) -> ResultSimulation<()>;
}

pub struct GenericArbitrageBot<S: ArbitrageStrategy> {
    pub common: CommonArbitrageBot,
    strategy: S,
}

impl<S: ArbitrageStrategy> GenericArbitrageBot<S> {
    pub fn new(
        ctx: &mut SimContext,
        mint_a: Pubkey,
        mint_b: Pubkey,
        initial_balance_a: u64,
        initial_balance_b: u64,
        config: ArbitrageConfig,
        seed: u64,
        strategy: S,
    ) -> ResultSimulation<Self> {
        let sim_user = SimUser::new(
            ctx,
            mint_a,
            mint_b,
            initial_balance_a,
            initial_balance_b,
            seed,
        )?;

        Ok(Self {
            common: CommonArbitrageBot::new(sim_user, config, seed),
            strategy,
        })
    }

    pub fn sim_user(&self) -> &SimUser {
        &self.common.sim_user
    }

    pub fn find_opportunity_for_target(
        &mut self,
        env: &mut S::Environment,
        strategy_data: S::StrategyData,
        market_price: f64,
        balance_a: u64,
        balance_b: u64,
        fee_rate: &FeeOption,
    ) -> ResultSimulation<Option<ArbitrageOpportunityGeneric<S::StrategyData>>> {
        let reserves = self.strategy.get_reserves(env, &strategy_data)?;

        let eval_calc = ProfitCalculator::new(market_price);

        let evaluate_direction = |is_a_to_b: bool,
                                  reserves_for_dir: &S::ReserveType,
                                  balance: u64,|
         -> Option<ArbitrageOpportunityGeneric<S::StrategyData>> {
            if balance == 0 {
                return None;
            }

            let calculate_output = |amount_in: u64| {
                self.strategy
                    .calculate_output(amount_in, reserves_for_dir, fee_rate)
                    .ok()
            };

            // We need to use a closure wrapper to unify the types
            let eval_profit: Box<dyn Fn(u64) -> Option<(u64, f64)>> = Box::new(|amount_in: u64| -> Option<(u64, f64)> {
                if is_a_to_b {
                    eval_calc.profit_evaluator_a_to_b(&calculate_output)(amount_in)
                } else {
                    eval_calc.profit_evaluator_b_to_a(&calculate_output)(amount_in)
                }
            });

            // Convert amount_in to token A value terms for proper profit normalization
            let to_value_a: Box<dyn Fn(u64) -> f64> = Box::new(|amount_in: u64| -> f64 {
                if is_a_to_b {
                    amount_in as f64  // Already in token A
                } else {
                    amount_in as f64 / market_price  // Convert from token B to token A value
                }
            });

            let candidate = self.common.find_best_with_profit(balance, eval_profit, to_value_a)?;

            if candidate.profit_percentage < self.common.config.min_profit_threshold {
                return None;
            }

            Some(ArbitrageOpportunityGeneric {
                strategy_data: strategy_data.clone(),
                is_a_to_b,
                amount_in: candidate.amount_in,
                expected_output: candidate.expected_output,
                market_price,
                expected_profit: candidate.net_profit as u64,
                profit_percentage: candidate.profit_percentage,
            })
        };

        let a_to_b = evaluate_direction(true, &reserves, balance_a);
        
        let swapped_reserves = self.strategy.swap_reserves(&reserves);
        let b_to_a = evaluate_direction(false, &swapped_reserves, balance_b);

        let best = match (a_to_b, b_to_a) {
            (Some(a), Some(b)) => {
                if a.expected_profit >= b.expected_profit {
                    Some(a)
                } else {

                    Some(b)
                }
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        Ok(best)
    }

    pub fn execute_opportunity(
        &self,
        env: &mut S::Environment,
        opportunity: &ArbitrageOpportunityGeneric<S::StrategyData>,
    ) -> ResultSimulation<u64> {
        let actual_output = self.strategy.execute_swap(
            env,
            &self.common.sim_user,
            &opportunity.strategy_data,
            opportunity.amount_in,
            opportunity.is_a_to_b,
        )?;

        // REBALANCE: After arbitrage, trade on external market to return to 50/50
        // This is the key to preventing directional risk for arbitrageurs
        self.rebalance_after_arbitrage(env, opportunity.market_price)?;

        Ok(actual_output)
    }

    /// Rebalances arbitrageur portfolio to 50/50 after executing arbitrage.
    ///
    /// This simulates the critical second step of real-world arbitrage: after exploiting
    /// a price difference on the CPMM, the arbitrageur immediately trades on an external
    /// market to neutralize directional risk.
    ///
    /// # Why This Matters
    ///
    /// Without rebalancing, arbitrageurs would accumulate one-sided positions and face
    /// directional price risk. If market price moves against them, their arbitrage
    /// profits would be wiped out. By immediately rebalancing to 50/50, they lock in
    /// their arbitrage profit and remain market-neutral.
    ///
    /// # Fee Treatment
    ///
    /// See [`crate::utils::actors::account::SimUser::rebalance_to_5050`] for detailed
    /// explanation of why external market fees are not explicitly modeled. In short: we
    /// assume the external market has sufficient volatility that the fee cost is absorbed
    /// within normal price deviations that are already larger than typical trading fees.
    fn rebalance_after_arbitrage(
        &self,
        env: &mut S::Environment,
        market_price: f64,
    ) -> ResultSimulation<()> {
        // This method would need access to SimContext through the environment
        // For now, we'll add this as a trait requirement on ArbitrageStrategy
        self.strategy.rebalance_user(&self.common.sim_user, env, market_price)
    }
}

use litesvm::LiteSVM;

use crate::{
    error::ResultSimulation,
    utils::{
        actors::arbitrageur_bot::{
            manager::{ArbitrageManager, ManagedArbitrageBot},
            ArbitrageStats,
        },
        ix::context::SimContext,
    },
};

use super::env_driver::SimulationEnvDriver;

/// Hook for mapping shared state updates from the environment into manager state.
pub trait SharedStateUpdater {
    fn update_from_pairs(&mut self, pairs: &[(u8, u8)]);
}

impl SharedStateUpdater for () {
    fn update_from_pairs(&mut self, _pairs: &[(u8, u8)]) {}
}

impl SharedStateUpdater for Vec<(u8, u8)> {
    fn update_from_pairs(&mut self, pairs: &[(u8, u8)]) {
        self.clear();
        self.extend_from_slice(pairs);
    }
}

/// Trait for driving arbitrage managers in a generic way.
pub trait ArbitrageManagerDriver {
    type Env: SimulationEnvDriver;

    fn is_empty(&self) -> bool;
    fn execute_round(&mut self, env: &mut Self::Env, slot: u64) -> ResultSimulation<()>;
    fn total_balances(&self, svm: &LiteSVM) -> (u64, u64);
    fn stats(&self) -> ArbitrageStats;
    fn set_shared_state(&mut self, _pairs: &[(u8, u8)]) {}
    fn reset_lamport_balances(&mut self, ctx: &mut SimContext, lamports_per_bot: u64) -> ResultSimulation<()>;
}

impl<B> ArbitrageManagerDriver for ArbitrageManager<B>
where
    B: ManagedArbitrageBot,
    B::Environment: SimulationEnvDriver,
    B::SharedState: SharedStateUpdater,
{
    type Env = B::Environment;

    fn is_empty(&self) -> bool {
        self.bots.is_empty()
    }

    fn execute_round(&mut self, env: &mut Self::Env, slot: u64) -> ResultSimulation<()> {
        self.execute_round(env, slot).map(|_| ())
    }

    fn total_balances(&self, svm: &LiteSVM) -> (u64, u64) {
        self.get_total_balances(svm)
    }

    fn stats(&self) -> ArbitrageStats {
        self.stats().clone()
    }

    fn set_shared_state(&mut self, pairs: &[(u8, u8)]) {
        self.shared_state.update_from_pairs(pairs);
    }

    fn reset_lamport_balances(&mut self, ctx: &mut SimContext, lamports_per_bot: u64) -> ResultSimulation<()> {
        // Call the ArbitrageManager implementation directly
        ArbitrageManager::reset_lamport_balances(self, ctx, lamports_per_bot)
    }
}

use litesvm::LiteSVM;
use solana_sdk::pubkey::Pubkey;

use crate::{
    error::ResultSimulation,
    utils::{
        actors::account::SimUser,
        ix::context::SimContext,
    },
};

use super::{traits::ArbitrageBotFactory, ArbitrageConfig, ArbitrageStats};

/// Trait implemented by arbitrage bots so they can be driven by the generic manager.
pub trait ManagedArbitrageBot {
    type Environment;
    type Opportunity;
    /// Shared state owned by the manager and passed to every bot on each tick.
    type SharedState;

    fn check_and_execute(
        &mut self,
        env: &mut Self::Environment,
        current_slot: u64,
        shared: &Self::SharedState,
    ) -> ResultSimulation<Option<Self::Opportunity>>;

    fn sim_user(&self) -> &SimUser;

    /// Optional hook to mutate shared state after a successful arbitrage.
    fn on_success(shared: &mut Self::SharedState, _opportunity: &Self::Opportunity) {
        let _ = shared;
    }
}

pub struct ArbitrageManager<B: ManagedArbitrageBot> {
    pub bots: Vec<B>,
    pub stats: ArbitrageStats,
    pub shared_state: B::SharedState,
}

impl<B: ManagedArbitrageBot> ArbitrageManager<B> {
    pub fn new(shared_state: B::SharedState) -> Self {
        Self {
            bots: Vec::new(),
            stats: ArbitrageStats::new(),
            shared_state,
        }
    }

    pub fn add_bot(&mut self, bot: B) {
        self.bots.push(bot);
    }

    pub fn execute_round(
        &mut self,
        env: &mut B::Environment,
        current_slot: u64,
    ) -> ResultSimulation<Vec<B::Opportunity>> {
        let mut executed = Vec::new();

        for bot in &mut self.bots {
            match bot.check_and_execute(env, current_slot, &self.shared_state) {
                Ok(Some(opportunity)) => {
                    self.stats.opportunities_found += 1;
                    self.stats.arbitrages_executed += 1;
                    B::on_success(&mut self.shared_state, &opportunity);
                    executed.push(opportunity);
                }
                Ok(None) => {}
                Err(_) => {
                    self.stats.failed_arbitrages += 1;
                }
            }
        }

        Ok(executed)
    }

    pub fn get_bot_balances(&self, svm: &LiteSVM) -> Vec<(Pubkey, u64, u64)> {
        use solana_sdk::signer::Signer;

        self.bots
            .iter()
            .map(|bot| {
                let balances = bot.sim_user().get_balances(svm);
                (bot.sim_user().keypair.pubkey(), balances.0, balances.1)
            })
            .collect()
    }

    pub fn print_stats(&self) {
        tracing::info!("Arbitrage Statistics:");
        tracing::info!("  Opportunities found: {}", self.stats.opportunities_found);
        tracing::info!("  Arbitrages executed: {}", self.stats.arbitrages_executed);
        tracing::info!("  Failed arbitrages: {}", self.stats.failed_arbitrages);
        tracing::info!("  Success rate: {:.2}%", self.stats.success_rate() * 100.0);
        tracing::info!("  Active bots: {}", self.bots.len());
    }

    pub fn is_empty(&self) -> bool {
        self.bots.is_empty()
    }

    pub fn get_total_balances(&self, svm: &LiteSVM) -> (u64, u64) {
        self.bots.iter().fold((0u64, 0u64), |(sum_a, sum_b), bot| {
            let (a, b) = bot.sim_user().get_balances(svm);
            (sum_a.saturating_add(a), sum_b.saturating_add(b))
        })
    }

    pub fn stats(&self) -> &ArbitrageStats {
        &self.stats
    }

    pub fn bot_count(&self) -> usize {
        self.bots.len()
    }

    /// Reset lamport balances of all arbitrage bots to prevent lamport exhaustion
    /// during long-running simulations. Sets account lamports to target amount.
    pub fn reset_lamport_balances(&self, ctx: &mut SimContext, target_lamports: u64) -> ResultSimulation<()> {
        use solana_sdk::signer::Signer;

        for bot in &self.bots {
            let pubkey = bot.sim_user().keypair.pubkey();
            
            // Simply set account lamports to target amount
            if let Some(mut account) = ctx.svm.get_account(&pubkey) {
                account.lamports = target_lamports;
                ctx.svm.set_account(pubkey, account).map_err(|e| {
                    crate::error::SimulationError::SystemError(format!("Failed to reset bot lamports: {:?}", e))
                })?;
            } else {
                // If account doesn't exist, airdrop
                ctx.svm.airdrop(&pubkey, target_lamports).map_err(|e| {
                    crate::error::SimulationError::FailedTransactionMetadata(e)
                })?;
            }
        }
        Ok(())
    }
}

/// Create a manager with the default shared state of the underlying bot.
pub fn new_generic_manager<B>() -> ArbitrageManager<B>
where
    B: ManagedArbitrageBot,
    B::SharedState: Default,
{
    ArbitrageManager::new(Default::default())
}

/// Add a new arbitrageur constructed via the `ArbitrageBotFactory` contract.
pub fn add_generic_arbitrageur<B>(
    mgr: &mut ArbitrageManager<B>,
    ctx: &mut SimContext,
    mint_a: Pubkey,
    mint_b: Pubkey,
    initial_balance_a: u64,
    initial_balance_b: u64,
    config: ArbitrageConfig,
    seed: u64,
) -> ResultSimulation<()>
where
    B: ArbitrageBotFactory + ManagedArbitrageBot,
{
    let bot = B::create(
        ctx,
        mint_a,
        mint_b,
        initial_balance_a,
        initial_balance_b,
        config,
        seed,
    )?;

    mgr.add_bot(bot);
    Ok(())
}

/// Execute a simulation round for any arbitrage manager.
pub fn execute_generic_round<B: ManagedArbitrageBot>(
    mgr: &mut ArbitrageManager<B>,
    env: &mut B::Environment,
    current_slot: u64,
) -> ResultSimulation<Vec<B::Opportunity>> {
    mgr.execute_round(env, current_slot)
}

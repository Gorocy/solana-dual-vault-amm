use anyhow::{anyhow, Context, Result};
use solana_sdk::{clock::Clock, pubkey::Pubkey};

use crate::utils::{
    args::config_simulation::SimulationConfig,
    actors::user::user_manager::UserManager,
    enviroment::setup::{EnvMode, MarketSimulationEnvironment},
    ix::{context::SimContext, vault_utils::VaultUtils},
    stats::VaultReserve,
};

/// Minimal interface the simulation runner needs from an environment.
pub trait SimulationEnvDriver {
    fn slot_setup(
        &mut self,
        slot: u64,
        config: &SimulationConfig,
        user_manager: &mut UserManager,
    ) -> Result<Option<Vec<(u8, u8)>>>;

    fn advance_price(&mut self);
    fn collect_vault_reserves(&mut self) -> Result<Vec<VaultReserve>>;
    fn market_price(&self) -> f64;

    fn ctx(&self) -> &SimContext;
    fn ctx_mut(&mut self) -> &mut SimContext;
    fn registry(&self) -> Pubkey;
}

impl SimulationEnvDriver for MarketSimulationEnvironment {
    fn slot_setup(
        &mut self,
        _slot: u64,
        config: &SimulationConfig,
        user_manager: &mut UserManager,
    ) -> Result<Option<Vec<(u8, u8)>>> {
        if self.mode == EnvMode::Dual {
            let clock = self.ctx.svm.get_sysvar::<Clock>();
            let slot_clock = clock.slot;
            let vault_pairs = cpmm_rebalansing::get_vault_pairs_for_slot(slot_clock, config.vault_count)
                .map_err(|e| anyhow!("Failed to get vault pairs: {e:?}"))?;

            user_manager.vaults = vault_pairs.clone();
            return Ok(Some(vault_pairs));
        }

        Ok(None)
    }

    fn advance_price(&mut self) {
        self.market.next_price();
    }

    fn collect_vault_reserves(&mut self) -> Result<Vec<VaultReserve>> {
        let mut reserves = Vec::new();

        for (i, vault_pda) in self.vaults.iter().enumerate() {
            let (reserve_a, reserve_b) = VaultUtils::get_vault_reserves(
                &self.ctx,
                *vault_pda,
                self.mint_a,
                self.mint_b,
            )
            .context("Failed to get vault reserves")?;

            reserves.push(VaultReserve::new(i, reserve_a, reserve_b));
        }

        Ok(reserves)
    }

    fn market_price(&self) -> f64 {
        self.market.current_price()
    }

    fn ctx(&self) -> &SimContext {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut SimContext {
        &mut self.ctx
    }

    fn registry(&self) -> Pubkey {
        self.registry
    }
}

use crate::utils::actors::arbitrageur_bot::{
    manager::ArbitrageManager, single::bot::SingleArbitrageBot, ArbitrageOpportunityGeneric,
};

pub mod bot;

#[derive(Debug, Clone)]
pub struct SingleVaultData {
    pub vault_index: u8,
}
pub type SingleArbitrageOpportunity = ArbitrageOpportunityGeneric<SingleVaultData>;

/// Type alias for ArbitrageManager specialized for single vault bots.
pub type SingleArbitrageManager = ArbitrageManager<SingleArbitrageBot>;

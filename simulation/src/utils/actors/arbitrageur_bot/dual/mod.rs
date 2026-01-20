use crate::utils::actors::arbitrageur_bot::{ArbitrageOpportunityGeneric, dual::bot::DualArbitrageBot, manager::ArbitrageManager};

pub mod bot;


#[derive(Debug, Clone)]
pub struct DualVaultData {
    pub vault1_index: u8,
    pub vault2_index: u8,
}

pub type DualArbitrageOpportunity = ArbitrageOpportunityGeneric<DualVaultData>;

/// Type alias for ArbitrageManager specialized for dual vault bots.
pub type DualArbitrageManager = ArbitrageManager<DualArbitrageBot>;

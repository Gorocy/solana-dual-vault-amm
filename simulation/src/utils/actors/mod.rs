pub mod account;
pub mod arbitrageur_bot;
pub mod user;


#[derive(Debug, Clone)]
pub enum SwapStrategy {
    /// Transakcje tylko na pojedynczym vault
    SingleVault,
    /// Transakcje na dwóch vault jednocześnie (dual swap)
    DualVault,
}

#[derive(Debug, Clone, Copy)]
pub enum SwapDirection {
    AtoB,
    BtoA,
}

#[derive(Debug, Default)]
pub struct SwapResult {
    pub success: bool,
    pub amount_in: u64,
    pub amount_out: u64,
    pub compute_units: u64,
    pub is_a_to_b: bool,
    pub vault_indices: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct TradingRoundStats {
    pub successful_swaps: u32,
    pub failed_swaps: u32,
    pub total_volume_a: u64,
    pub total_volume_b: u64,
    pub single_vault_swaps: u32,
    pub dual_vault_swaps: u32,
}

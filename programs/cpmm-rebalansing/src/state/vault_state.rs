use anchor_lang::prelude::*;

#[account]
#[derive(Debug, InitSpace)]
pub struct VaultRegistry {
    pub token_a: Pubkey, // 32
    pub token_b: Pubkey, // 32
    pub fee: FeeOption,  // 1 - u64 would take 8
    pub bump: u8,        // 1
    pub vault_count: u8, // 1 - number of existing vaults
}

#[account]
#[derive(Debug, InitSpace)]
pub struct Vault {
    pub vault_index: u8,  // 1 - index of this vault in registry
    pub bump: u8,         // 1
    pub lp_bump: u8,      // 1
    pub registry: Pubkey, // 32
}

#[derive(Debug, InitSpace, Clone, AnchorSerialize, AnchorDeserialize)]
pub enum FeeOption {
    Tier30,
    Tier25,
    Tier20,
}

impl FeeOption {
    /// fee in basis points
    pub fn fees(&self) -> (u64, u64) {
        match self {
            FeeOption::Tier30 => (30, 25),
            FeeOption::Tier25 => (25, 21),
            FeeOption::Tier20 => (20, 16),
        }
    }

    pub fn normal(&self) -> u64 {
        self.fees().0
    }

    pub fn discount(&self) -> u64 {
        self.fees().1
    }
}

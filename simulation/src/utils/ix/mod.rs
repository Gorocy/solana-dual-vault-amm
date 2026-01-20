use borsh::{BorshDeserialize, BorshSerialize};
use litesvm::LiteSVM;
use serde::{Deserialize, Serialize};
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

use crate::error::ResultSimulation;

pub mod context;
pub mod dual_swap_utils;
pub mod manage_liqudity;
pub mod registry_utils;
pub mod single_swap_utils;
pub mod token_utils;
pub mod vault_utils;

use spl_token::state::Account as SplTokenAccount;

/// Helper do odczytu balansu tokenów z LiteSVM
pub fn get_token_balance(svm: &LiteSVM, token_account: Pubkey) -> ResultSimulation<u64> {
    match svm.get_account(&token_account) {
        Some(account) => {
            let token_acc = SplTokenAccount::unpack(&account.data).map_err(|e| {
                crate::error::SimulationError::SystemError(format!(
                    "Failed to unpack token account: {}",
                    e
                ))
            })?;
            Ok(token_acc.amount)
        }
        None => Ok(0),
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct VaultRegistry {
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub fee: FeeOption,
    pub bump: u8,
    pub vault_count: u8,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum FeeOption {
    Tier30,
    Tier25,
    Tier20,
}

impl FeeOption {
    pub fn new(fee: u64) -> Self {
        match fee {
            30 => FeeOption::Tier30,
            25 => FeeOption::Tier25,
            20 => FeeOption::Tier20,
            _ => panic!("Invalid fee"),
        }
    }

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

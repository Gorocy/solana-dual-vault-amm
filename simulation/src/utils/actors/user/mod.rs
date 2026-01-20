pub mod behavior;
pub mod user_batch;
pub mod user_manager;
mod utils;
pub(super) mod dual;
pub(super) mod single;

pub use behavior::{UserBehaviorConfig, UserBehaviorConfigBuilder};
use rand::rngs::StdRng;
use solana_sdk::pubkey::Pubkey;

use crate::{error::ResultSimulation, utils::{actors::{SwapResult, account::SimUser}, ix::context::SimContext}};

pub(super) trait UserSwapStrategy {
    fn ensure_vaults_available(
        &self,
        vault_count: u8,
        vault_pairs: &[(u8, u8)],
    ) -> ResultSimulation<()>;

    fn pick_vault_index(
        &self,
        rng: &mut StdRng,
        vault_count: u8,
        vault_pairs: &[(u8, u8)],
    ) -> Option<usize>;

    fn execute_user_swap(
        &self,
        ctx: &mut SimContext,
        user: &SimUser,
        registry_pubkey: Pubkey,
        amount_in: u64,
        is_a_to_b: bool,
        vault_index: usize,
        vault_pairs: &[(u8, u8)],
    ) -> ResultSimulation<SwapResult>;
}

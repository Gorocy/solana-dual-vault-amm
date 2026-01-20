use solana_sdk::pubkey::Pubkey;
use tracing::error;

use super::utils::ManagerUtils;
use crate::{
    error::{ResultSimulation, SimulationError},
    utils::{
        actors::{account::SimUser, user::UserSwapStrategy, SwapResult},
        ix::{context::SimContext, single_swap_utils::SingleSwapUtils},
    },
};
use rand::rngs::StdRng;

pub(super) struct SingleVaultStrategy;

impl UserSwapStrategy for SingleVaultStrategy {
    fn ensure_vaults_available(
        &self,
        vault_count: u8,
        _vault_pairs: &[(u8, u8)],
    ) -> ResultSimulation<()> {
        if vault_count == 0 {
            Err(SimulationError::SystemError(
                "Not enough vaults".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    fn pick_vault_index(
        &self,
        rng: &mut StdRng,
        vault_count: u8,
        _vault_pairs: &[(u8, u8)],
    ) -> Option<usize> {
        ManagerUtils::pick_single_vault_index(rng, vault_count)
    }

    fn execute_user_swap(
        &self,
        ctx: &mut SimContext,
        user: &SimUser,
        registry_pubkey: Pubkey,
        amount_in: u64,
        is_a_to_b: bool,
        vault_index: usize,
        _vault_pairs: &[(u8, u8)],
    ) -> ResultSimulation<SwapResult> {
        let minimum_amount_out = 1;

        match SingleSwapUtils::execute_single_swap_with_compute(
            ctx,
            Some(&user.keypair),
            registry_pubkey,
            vault_index as u8,
            amount_in,
            minimum_amount_out,
            is_a_to_b,
        ) {
            Ok(exec) => Ok(SwapResult {
                success: true,
                amount_in,
                amount_out: exec.amount_out,
                compute_units: exec.compute_units,
                is_a_to_b,
                vault_indices: vec![vault_index as u8],
            }),
            Err(e) => {
                error!("Single vault swap failed: {:?}", e);
                Ok(SwapResult {
                    success: false,
                    ..Default::default()
                })
            }
        }
    }
}

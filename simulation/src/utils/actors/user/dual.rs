use solana_sdk::{pubkey::Pubkey};
use tracing::{error};

use crate::{
    error::{ResultSimulation, SimulationError},
    utils::{
        actors::{
            SwapResult, account::SimUser, user::UserSwapStrategy,
        },
        ix::{
            context::SimContext, dual_swap_utils::DualSwapUtils,
        },
    },
};
use super::utils::ManagerUtils;
use rand::{rngs::StdRng};

pub(super) struct DualVaultStrategy;

impl UserSwapStrategy for DualVaultStrategy {
    fn ensure_vaults_available(
        &self,
        _vault_count: u8,
        vault_pairs: &[(u8, u8)],
    ) -> ResultSimulation<()> {
        if vault_pairs.is_empty() {
            Err(SimulationError::SystemError(
                "No dual vault pairs configured for user simulator".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    fn pick_vault_index(
        &self,
        rng: &mut StdRng,
        _vault_count: u8,
        vault_pairs: &[(u8, u8)],
    ) -> Option<usize> {
        ManagerUtils::pick_dual_vault_index(rng, vault_pairs)
    }

    fn execute_user_swap(
        &self,
        ctx: &mut SimContext,
        user: &SimUser,
        registry_pubkey: Pubkey,
        amount_in: u64,
        is_a_to_b: bool,
        vault_index: usize,
        vault_pairs: &[(u8, u8)],
    ) -> ResultSimulation<SwapResult> {
        if vault_index >= vault_pairs.len() {
            return Err(SimulationError::SystemError("Not enough vault pairs".to_string()));
        }

        let (vault1_index, vault2_index) = vault_pairs[vault_index];

        let minimum_amount_out = 1;

        match DualSwapUtils::execute_dual_swap_with_compute(
            ctx,
            Some(&user.keypair),
            registry_pubkey,
            vault1_index,
            vault2_index,
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
                vault_indices: vec![vault1_index, vault2_index],
            }),
            Err(e) => {
                error!("Dual vault swap failed: {:?}", e);
                Ok(SwapResult {
                    success: false,
                    ..Default::default()
                })
            }
        }
    }
}

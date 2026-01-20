use crate::utils::ix::context::SimContext;
use crate::utils::ix::{get_token_balance, VaultRegistry};
use crate::{anchor_discriminator, error::ResultSimulation, PROGRAM_ID};

use borsh::BorshDeserialize;
use cpmm_rebalansing::VAULT_SEED;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use tracing::{debug, trace};

pub struct SingleSwapUtils;

pub struct SwapExecution {
    pub amount_out: u64,
    pub compute_units: u64,
}

impl SingleSwapUtils {
    /// Executes a single vault swap.
    ///
    /// If `signer` is `None`, the context payer is used as the user/authority.
    pub fn execute_single_swap(
        ctx: &mut SimContext,
        signer: Option<&solana_sdk::signature::Keypair>,
        registry_pubkey: Pubkey,
        vault_index: u8,
        amount_in: u64,
        minimum_amount_out: u64,
        is_a_to_b: bool,
    ) -> ResultSimulation<u64> {
        let exec = Self::execute_single_swap_with_compute(
            ctx,
            signer,
            registry_pubkey,
            vault_index,
            amount_in,
            minimum_amount_out,
            is_a_to_b,
        )?;

        Ok(exec.amount_out)
    }

    pub fn execute_single_swap_with_compute(
        ctx: &mut SimContext,
        signer: Option<&solana_sdk::signature::Keypair>,
        registry_pubkey: Pubkey,
        vault_index: u8,
        amount_in: u64,
        minimum_amount_out: u64,
        is_a_to_b: bool,
    ) -> ResultSimulation<SwapExecution> {
        let registry_account = ctx.svm.get_account(&registry_pubkey).ok_or_else(|| {
            crate::error::SimulationError::SystemError("Registry not found".into())
        })?;

        if registry_account.data.len() < 8 {
            return Err(crate::error::SimulationError::SystemError(
                "Registry account too small".into(),
            ));
        }

        let registry: VaultRegistry = VaultRegistry::try_from_slice(&registry_account.data[8..])
            .map_err(|e| {
                crate::error::SimulationError::SystemError(format!(
                    "Failed to deserialize registry: {e}"
                ))
            })?;

        let mint_a = registry.token_a;
        let mint_b = registry.token_b;

        // ─────────────────────────────────────────────
        let (vault_pda, _) = Pubkey::find_program_address(
            &[VAULT_SEED, registry_pubkey.as_ref(), &[vault_index]],
            &PROGRAM_ID,
        );

        // Resolve user (logical signer or payer)
        let user_keypair = if signer.is_some() {
            signer.unwrap().insecure_clone()
        } else {
            let keypair = ctx.payer.insecure_clone();
            keypair
        };
        let user_pubkey = user_keypair.pubkey();

        // Vault ATAs
        let vault_token_a = get_associated_token_address(&vault_pda, &mint_a);
        let vault_token_b = get_associated_token_address(&vault_pda, &mint_b);

        // User ATAs
        let user_token_a = get_associated_token_address(&user_pubkey, &mint_a);
        let user_token_b = get_associated_token_address(&user_pubkey, &mint_b);

        // ─────────────────────────────────────────────
        // 3️⃣ Determine output token based on swap direction
        // ─────────────────────────────────────────────
        let user_out_token_pubkey = if is_a_to_b {
            user_token_b
        } else {
            user_token_a
        };

        // ─────────────────────────────────────────────
        // 4️⃣ Snapshot user balance BEFORE swap
        // ─────────────────────────────────────────────
        let balance_before = get_token_balance(&ctx.svm, user_out_token_pubkey)?;

        // ─────────────────────────────────────────────
        // 5️⃣ Build instruction accounts (Anchor order)
        // ─────────────────────────────────────────────
        let accounts = vec![
            // 0. user / authority
            AccountMeta::new(user_pubkey, true),
            // 1. vault registry
            AccountMeta::new_readonly(registry_pubkey, false),
            // 2. vault
            AccountMeta::new(vault_pda, false),
            // 3. vault_token_in
            AccountMeta::new(
                if is_a_to_b {
                    vault_token_a
                } else {
                    vault_token_b
                },
                false,
            ),
            // 4. vault_token_out
            AccountMeta::new(
                if is_a_to_b {
                    vault_token_b
                } else {
                    vault_token_a
                },
                false,
            ),
            // 5. user_token_in
            AccountMeta::new(
                if is_a_to_b {
                    user_token_a
                } else {
                    user_token_b
                },
                false,
            ),
            // 6. user_token_out
            AccountMeta::new(
                if is_a_to_b {
                    user_token_b
                } else {
                    user_token_a
                },
                false,
            ),
            // 7. token_in_mint
            AccountMeta::new_readonly(if is_a_to_b { mint_a } else { mint_b }, false),
            // 8. token_out_mint
            AccountMeta::new_readonly(if is_a_to_b { mint_b } else { mint_a }, false),
            // 9. token program
            AccountMeta::new_readonly(spl_token::id(), false),
        ];
        trace!("SWAP");

        // ─────────────────────────────────────────────
        let mut data = anchor_discriminator("swap_single").to_vec();
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&minimum_amount_out.to_le_bytes());

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data,
        };

        // ─────────────────────────────────────────────
        #[cfg(feature = "payer-sign")]
        {
            let payer = &mut ctx.payer.insecure_clone();
            let compute_units = ctx.send_tx(&[ix], Some(&[&user_keypair, &payer]))?;

            let balance_after = get_token_balance(&ctx.svm, user_out_token_pubkey)?;
            let actual_amount_out = balance_after.saturating_sub(balance_before);

            debug!(
                "Single swap executed: in={}, out={} (min expected={})",
                amount_in, actual_amount_out, minimum_amount_out,
            );

            return Ok(SwapExecution {
                amount_out: actual_amount_out,
                compute_units,
            });
        }
        #[cfg(not(feature = "payer-sign"))]
        {
            let compute_units = ctx.send_tx(&[ix], Some(&[&user_keypair]))?;

            let balance_after = get_token_balance(&ctx.svm, user_out_token_pubkey)?;
            let actual_amount_out = balance_after.saturating_sub(balance_before);

            debug!(
                "Single swap executed: in={}, out={} (min expected={})",
                amount_in, actual_amount_out, minimum_amount_out,
            );

            return Ok(SwapExecution {
                amount_out: actual_amount_out,
                compute_units,
            });
        }
    }
}

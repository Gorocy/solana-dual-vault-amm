use crate::{
    anchor_discriminator,
    error::ResultSimulation,
    utils::ix::{context::SimContext, get_token_balance, VaultRegistry},
    PROGRAM_ID,
};

use borsh::BorshDeserialize;
use cpmm_rebalansing::VAULT_SEED;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
    sysvar,
};
use spl_associated_token_account::get_associated_token_address;
use tracing::trace;

pub struct DualSwapUtils;

pub struct DualSwapExecution {
    pub amount_out: u64,
    pub compute_units: u64,
}

impl DualSwapUtils {
    /// Executes a deterministic dual swap between two vaults.
    ///
    /// If `signer` is `None`, the context payer is used as the user/authority.
    pub fn execute_dual_swap(
        ctx: &mut SimContext,
        signer: Option<&solana_sdk::signature::Keypair>,
        registry_pubkey: Pubkey,
        vault1_index: u8,
        vault2_index: u8,
        amount_in: u64,
        minimum_amount_out: u64,
        is_a_to_b: bool,
    ) -> ResultSimulation<u64> {
        let exec = Self::execute_dual_swap_with_compute(
            ctx,
            signer,
            registry_pubkey,
            vault1_index,
            vault2_index,
            amount_in,
            minimum_amount_out,
            is_a_to_b,
        )?;

        Ok(exec.amount_out)
    }

    pub fn execute_dual_swap_with_compute(
        ctx: &mut SimContext,
        signer: Option<&solana_sdk::signature::Keypair>,
        registry_pubkey: Pubkey,
        vault1_index: u8,
        vault2_index: u8,
        amount_in: u64,
        minimum_amount_out: u64,
        is_a_to_b: bool,
    ) -> ResultSimulation<DualSwapExecution> {
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
        let (vault1_pda, _) = Pubkey::find_program_address(
            &[VAULT_SEED, registry_pubkey.as_ref(), &[vault1_index]],
            &PROGRAM_ID,
        );

        let (vault2_pda, _) = Pubkey::find_program_address(
            &[VAULT_SEED, registry_pubkey.as_ref(), &[vault2_index]],
            &PROGRAM_ID,
        );

        // ─────────────────────────────────────────────
        let user_keypair = if signer.is_some() {
            signer.unwrap().insecure_clone()
        } else {
            let keypair = ctx.payer.insecure_clone();
            keypair
        };
        let user_pubkey = user_keypair.pubkey();

        // Associated Token Accounts (vaults)
        let vault1_token_a = get_associated_token_address(&vault1_pda, &mint_a);
        let vault1_token_b = get_associated_token_address(&vault1_pda, &mint_b);
        let vault2_token_a = get_associated_token_address(&vault2_pda, &mint_a);
        let vault2_token_b = get_associated_token_address(&vault2_pda, &mint_b);

        // Associated Token Accounts (user)
        let user_token_a = get_associated_token_address(&user_pubkey, &mint_a);
        let user_token_b = get_associated_token_address(&user_pubkey, &mint_b);

        // ─────────────────────────────────────────────
        let (token_in_mint, token_out_mint, user_token_in, user_token_out) = if is_a_to_b {
            (mint_a, mint_b, user_token_a, user_token_b)
        } else {
            (mint_b, mint_a, user_token_b, user_token_a)
        };

        let (vault1_token_in, vault1_token_out) = if is_a_to_b {
            (vault1_token_a, vault1_token_b)
        } else {
            (vault1_token_b, vault1_token_a)
        };

        let (vault2_token_in, vault2_token_out) = if is_a_to_b {
            (vault2_token_a, vault2_token_b)
        } else {
            (vault2_token_b, vault2_token_a)
        };

        // ─────────────────────────────────────────────
        let balance_before = get_token_balance(&ctx.svm, user_token_out)?;

        let clock_pubkey = sysvar::clock::id();

        trace!("Prepare accounts");

        // ─────────────────────────────────────────────
        let accounts = vec![
            AccountMeta::new(user_pubkey, true), // user / authority
            AccountMeta::new_readonly(registry_pubkey, false),
            AccountMeta::new(vault1_pda, false),
            AccountMeta::new(vault2_pda, false),
            AccountMeta::new(vault1_token_in, false),
            AccountMeta::new(vault1_token_out, false),
            AccountMeta::new(vault2_token_in, false),
            AccountMeta::new(vault2_token_out, false),
            AccountMeta::new(user_token_in, false),
            AccountMeta::new(user_token_out, false),
            AccountMeta::new_readonly(token_in_mint, false),
            AccountMeta::new_readonly(token_out_mint, false),
            AccountMeta::new_readonly(clock_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        // ─────────────────────────────────────────────
        let mut data = anchor_discriminator("swap_dual_deterministic").to_vec();
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&minimum_amount_out.to_le_bytes());

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data,
        };

        // ─────────────────────────────────────────────
        let compute_units = ctx.send_tx(&[ix], Some(&[&user_keypair]))?;

        // ─────────────────────────────────────────────
        let balance_after = get_token_balance(&ctx.svm, user_token_out)?;
        let actual_amount_out = balance_after.saturating_sub(balance_before);

        trace!(
            "Dual swap executed: in={}, out={} (min expected={})",
            amount_in, actual_amount_out, minimum_amount_out,
        );

        Ok(DualSwapExecution {
            amount_out: actual_amount_out,
            compute_units,
        })
    }
}

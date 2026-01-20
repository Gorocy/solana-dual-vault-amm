use borsh::BorshDeserialize;
use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

use crate::{
    anchor_discriminator,
    error::{ResultSimulation, SimulationError},
    utils::ix::{context::SimContext, token_utils::TokenUtils, VaultRegistry},
    PROGRAM_ID,
};

use cpmm_rebalansing::{LP_TOKEN_SEED, VAULT_SEED};
use spl_associated_token_account::get_associated_token_address;

pub struct ManageLiquidityInstruction;

impl ManageLiquidityInstruction {
    /// Adds liquidity to a vault.
    /// If `signer` is `None`, the context payer is used as the user.
    pub fn add_liquidity_to_vault(
        ctx: &mut SimContext,
        signer: Option<&solana_sdk::signature::Keypair>,
        registry_pubkey: Pubkey,
        vault_index: u8,
        amount_a: u64,
        amount_b: u64,
        min_lp_amount: u64,
    ) -> ResultSimulation<()> {
        // ─────────────────────────────────────────────
        // 1️⃣ Read registry account (borsh)
        // ─────────────────────────────────────────────
        let registry_account = ctx.svm.get_account(&registry_pubkey).ok_or_else(|| {
            SimulationError::SystemError("Registry account not found".to_string())
        })?;

        if registry_account.data.len() < 8 {
            return Err(SimulationError::SystemError(
                "Registry account too small".to_string(),
            ));
        }

        let registry: VaultRegistry = VaultRegistry::try_from_slice(&registry_account.data[8..])
            .map_err(|e| {
                SimulationError::SystemError(format!("Failed to deserialize registry: {e}"))
            })?;

        let mint_a = registry.token_a;
        let mint_b = registry.token_b;

        // ─────────────────────────────────────────────
        // 2️⃣ Derive vault PDA (vault index = 1 byte)
        // ─────────────────────────────────────────────
        let (vault_pda, _) = Pubkey::find_program_address(
            &[VAULT_SEED, registry_pubkey.as_ref(), &[vault_index]],
            &PROGRAM_ID,
        );

        let (lp_mint_pda, _) =
            Pubkey::find_program_address(&[LP_TOKEN_SEED, vault_pda.as_ref()], &PROGRAM_ID);

        // ─────────────────────────────────────────────
        // 3️⃣ Resolve user (signer or payer)
        // ─────────────────────────────────────────────
        let user_keypair = if signer.is_some() {
            signer.unwrap().insecure_clone()
        } else {
            let keypair = ctx.payer.insecure_clone();
            keypair
        };
        let user = user_keypair.pubkey();

        #[cfg(feature = "create-user-lp-token-account")]
        {
            TokenUtils::create_token_account(ctx, &lp_mint_pda, user, 0)?;
        }

        // Associated Token Accounts
        let user_token_a = get_associated_token_address(&user, &mint_a);
        let user_token_b = get_associated_token_address(&user, &mint_b);
        let user_lp_token = get_associated_token_address(&user, &lp_mint_pda);

        let vault_token_a = get_associated_token_address(&vault_pda, &mint_a);
        let vault_token_b = get_associated_token_address(&vault_pda, &mint_b);

        // ─────────────────────────────────────────────
        // 4️⃣ Accounts (must match Anchor context)
        // ─────────────────────────────────────────────
        let accounts = vec![
            AccountMeta::new(user, true), // user / authority
            AccountMeta::new_readonly(registry_pubkey, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new(vault_token_a, false),
            AccountMeta::new(vault_token_b, false),
            AccountMeta::new(user_token_a, false),
            AccountMeta::new(user_token_b, false),
            AccountMeta::new(user_lp_token, false),
            AccountMeta::new(lp_mint_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        // Instruction data
        let mut data = anchor_discriminator("add_liquidity").to_vec();
        data.extend_from_slice(&amount_a.to_le_bytes());
        data.extend_from_slice(&amount_b.to_le_bytes());
        data.extend_from_slice(&min_lp_amount.to_le_bytes());

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data,
        };

        ctx.send_tx(&[ix], Some(&[&user_keypair]))?;
        Ok(())
    }

    /// Removes liquidity from a vault.
    /// If `signer` is `None`, the context payer is used as the user.
    pub fn remove_liquidity_from_vault(
        ctx: &mut SimContext,
        signer: Option<&solana_sdk::signature::Keypair>,
        registry_pubkey: Pubkey,
        vault_index: u8,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> ResultSimulation<()> {
        let (mint_a, mint_b) = {
            let registry_account = ctx.svm.get_account(&registry_pubkey).ok_or_else(|| {
                SimulationError::SystemError("Registry account not found".to_string())
            })?;

            let registry: VaultRegistry =
                VaultRegistry::try_from_slice(&registry_account.data[8..]).map_err(|e| {
                    SimulationError::SystemError(format!("Failed to deserialize registry: {e}"))
                })?;

            let mint_a = registry.token_a;
            let mint_b = registry.token_b;
            (mint_a, mint_b)
        };

        // ─────────────────────────────────────────────
        // 2️⃣ PDAs
        // ─────────────────────────────────────────────
        let (vault_pda, _) = Pubkey::find_program_address(
            &[VAULT_SEED, registry_pubkey.as_ref(), &[vault_index]],
            &PROGRAM_ID,
        );

        let (lp_mint_pda, _) =
            Pubkey::find_program_address(&[LP_TOKEN_SEED, vault_pda.as_ref()], &PROGRAM_ID);

        // ─────────────────────────────────────────────
        // 3️⃣ Resolve user
        // ─────────────────────────────────────────────

        let (user, user_keypair) = {
            let user_keypair = if signer.is_some() {
                signer.unwrap().insecure_clone()
            } else {
                let keypair = ctx.payer.insecure_clone();
                keypair
            };
            (user_keypair.pubkey(), user_keypair)
        };

        let user_token_a = get_associated_token_address(&user, &mint_a);
        let user_token_b = get_associated_token_address(&user, &mint_b);
        let user_lp_token = get_associated_token_address(&user, &lp_mint_pda);

        let vault_token_a = get_associated_token_address(&vault_pda, &mint_a);
        let vault_token_b = get_associated_token_address(&vault_pda, &mint_b);

        // ─────────────────────────────────────────────
        // 4️⃣ Accounts (must match Anchor context)
        // ─────────────────────────────────────────────
        let accounts = vec![
            AccountMeta::new(user, true), // user / authority
            AccountMeta::new_readonly(registry_pubkey, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new(vault_token_a, false),
            AccountMeta::new(vault_token_b, false),
            AccountMeta::new(user_token_a, false),
            AccountMeta::new(user_token_b, false),
            AccountMeta::new(user_lp_token, false),
            AccountMeta::new(lp_mint_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let mut data = anchor_discriminator("remove_liquidity").to_vec();
        data.extend_from_slice(&lp_amount.to_le_bytes());
        data.extend_from_slice(&min_amount_a.to_le_bytes());
        data.extend_from_slice(&min_amount_b.to_le_bytes());

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data,
        };

        ctx.send_tx(&[ix], Some(&[&user_keypair]))?;
        Ok(())
    }
}

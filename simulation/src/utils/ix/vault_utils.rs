use borsh::BorshDeserialize;
use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signer::Signer,
};
use tracing::debug;

use crate::{
    anchor_discriminator,
    error::ResultSimulation,
    utils::ix::{context::SimContext, VaultRegistry},
    PROGRAM_ID,
};
use cpmm_rebalansing::{LP_TOKEN_SEED, VAULT_SEED};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account as TokenAccount;

pub struct VaultUtils;

impl VaultUtils {
    pub fn init_vault(
        ctx: &mut SimContext,
        program_id: Pubkey,
        registry: Pubkey,
        // Usunęliśmy argument `current_vault_count` - funkcja sama go sobie znajdzie
    ) -> ResultSimulation<Pubkey> {
        // KROK A: Pobierz surowe dane konta Registry z SVM
        let registry_account = ctx.svm.get_account(&registry).ok_or_else(|| {
            crate::error::SimulationError::SystemError("Registry account not found".to_string())
        })?;

        let data = &registry_account.data;

        if data.len() < 8 {
            return Err(crate::error::SimulationError::SystemError(format!(
                "Account too small"
            )));
        }

        let registry_state = VaultRegistry::try_from_slice(&data[8..]).map_err(|e| {
            crate::error::SimulationError::SystemError(format!(
                "Failed to deserialize registry: {}",
                e
            ))
        })?;

        // KROK C: Wyciągnij aktualny licznik
        // W Twoim kodzie Anchor: vault_registry.vault_count jest używany jako seed PRZED inkrementacją
        let current_vault_count = registry_state.vault_count;

        debug!(
            "Fetching registry state. Current vault count: {}",
            current_vault_count
        );

        // 1. PDA vault
        let (vault_pda, _) = Pubkey::find_program_address(
            &[
                VAULT_SEED, // Zgodnie z Twoim kodem Anchor
                registry.as_ref(),
                &[current_vault_count], // ← KLUCZOWE: 1 BAJT
            ],
            &PROGRAM_ID,
        );

        // 2. PDA LP mint
        let (mint_lp_pda, _) =
            Pubkey::find_program_address(&[LP_TOKEN_SEED, vault_pda.as_ref()], &program_id);

        // Musimy wiedzieć, jakie są minty w rejestrze, żeby wyliczyć ATA
        // Na szczęście mamy już `registry_state`, więc możemy je stamtąd wziąć!
        let mint_a = registry_state.token_a;
        let mint_b = registry_state.token_b;

        // 3. ATAs
        let vault_token_a = get_associated_token_address(&vault_pda, &mint_a);
        let vault_token_b = get_associated_token_address(&vault_pda, &mint_b);

        // 4. Accounts
        // Kolejność musi być IDENTYCZNA jak w struct InitializeVault
        let accounts = vec![
            AccountMeta::new(ctx.payer.pubkey(), true), // payer
            AccountMeta::new(registry, false),          // vault_registry
            AccountMeta::new(vault_pda, false),         // vault
            AccountMeta::new_readonly(mint_a, false),   // mint_a (z registry)
            AccountMeta::new_readonly(mint_b, false),   // mint_b (z registry)
            AccountMeta::new(mint_lp_pda, false),       // mint_lp
            AccountMeta::new(vault_token_a, false),     // vault_token_a
            AccountMeta::new(vault_token_b, false),     // vault_token_b
            // Programy
            AccountMeta::new_readonly(spl_token::id(), false), // token_program
            // Jeśli używasz token-2022 w conditional compilation, tutaj może być potrzebna logika,
            // ale dla standardowych testów zazwyczaj wystarcza spl_token::id() jeśli minty są standardowe.
            AccountMeta::new_readonly(spl_associated_token_account::id(), false), // associated_token_program
            AccountMeta::new_readonly(system_program::id(), false),               // system_program
        ];

        // 5. Instruction data
        let data = anchor_discriminator("initialize_vault").to_vec();

        let ix = Instruction {
            program_id,
            accounts,
            data,
        };

        // 6. Send tx
        ctx.send_tx(&[ix], None)?;

        Ok(vault_pda)
    }

    pub fn get_vault_reserves(
        ctx: &SimContext,
        vault_pubkey: Pubkey,
        mint_a: Pubkey,
        mint_b: Pubkey,
    ) -> ResultSimulation<(u64, u64)> {
        let vault_token_a = get_associated_token_address(&vault_pubkey, &mint_a);
        let vault_token_b = get_associated_token_address(&vault_pubkey, &mint_b);

        let account_a = ctx.svm.get_account(&vault_token_a).ok_or_else(|| {
            crate::error::SimulationError::SystemError("Token account A not found".to_string())
        })?;

        let account_b = ctx.svm.get_account(&vault_token_b).ok_or_else(|| {
            crate::error::SimulationError::SystemError("Token account B not found".to_string())
        })?;

        let token_account_a = TokenAccount::unpack(&account_a.data).map_err(|e| {
            crate::error::SimulationError::SystemError(format!(
                "Failed to unpack token account A: {}",
                e
            ))
        })?;
        let token_account_b = TokenAccount::unpack(&account_b.data).map_err(|e| {
            crate::error::SimulationError::SystemError(format!(
                "Failed to unpack token account B: {}",
                e
            ))
        })?;

        Ok((token_account_a.amount, token_account_b.amount))
    }

    pub fn get_vault_token_account(vault_pubkey: Pubkey, mint: Pubkey) -> ResultSimulation<Pubkey> {
        // Return associated token account for the vault
        Ok(get_associated_token_address(&vault_pubkey, &mint))
    }

    pub fn get_vault_lp_mint(vault_pubkey: Pubkey) -> ResultSimulation<Pubkey> {
        // Calculate LP mint PDA
        let (lp_mint_pda, _) =
            Pubkey::find_program_address(&[LP_TOKEN_SEED, vault_pubkey.as_ref()], &PROGRAM_ID);

        Ok(lp_mint_pda)
    }
}

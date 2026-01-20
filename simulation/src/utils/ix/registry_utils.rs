use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
};

use cpmm_rebalansing::constants::VAULT_REGISTRY_SEED;

use crate::{
    anchor_discriminator,
    error::ResultSimulation,
    utils::ix::{context::SimContext, FeeOption},
    PROGRAM_ID,
};

pub struct RegistryUtils;

impl RegistryUtils {
    pub fn initialize_vault_registry(
        ctx: &mut SimContext,
        mint_a: Pubkey,
        mint_b: Pubkey,
        fee: FeeOption,
    ) -> ResultSimulation<Pubkey> {
        let (vault_registry_pda, _) = Pubkey::find_program_address(
            &[VAULT_REGISTRY_SEED, mint_a.as_ref(), mint_b.as_ref()],
            &PROGRAM_ID,
        );

        let accounts = vec![
            AccountMeta::new(ctx.payer.pubkey(), true),
            AccountMeta::new(vault_registry_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let mut data = anchor_discriminator("initialize_vault_registry").to_vec();
        data.extend_from_slice(&mint_a.to_bytes());
        data.extend_from_slice(&mint_b.to_bytes());
        data.extend_from_slice(&[match fee {
            FeeOption::Tier30 => 0,
            FeeOption::Tier25 => 1,
            FeeOption::Tier20 => 2,
        }]);

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data,
        };

        ctx.send_tx(&[ix], None)?;
        Ok(vault_registry_pda)
    }
}

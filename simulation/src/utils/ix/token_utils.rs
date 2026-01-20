use cpmm_rebalansing::DECIMALS;
use solana_program::example_mocks::solana_sdk::system_instruction;
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;

use crate::{error::ResultSimulation, utils::ix::context::SimContext};

#[derive(Clone, Copy)]
pub struct TokenPair {
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
}

pub struct TokenUtils;

impl TokenUtils {
    pub fn setup_sorted_mints(ctx: &mut SimContext) -> TokenPair {
        let (mint_a, _) = Self::create_mint(ctx, DECIMALS).expect("Failed to create mint A");
        let (mint_b, _) = Self::create_mint(ctx, DECIMALS).expect("Failed to create mint B");

        // Wymóg kontraktu: mint_a > mint_b
        let (mint_a, mint_b) = if mint_a > mint_b {
            (mint_a, mint_b)
        } else {
            (mint_b, mint_a)
        };

        TokenPair { mint_a, mint_b }
    }

    pub fn create_mint(ctx: &mut SimContext, decimals: u8) -> ResultSimulation<(Pubkey, Keypair)> {
        let mint_kp = Keypair::new();
        let payer = ctx.payer.pubkey();
        let mint = mint_kp.pubkey();

        let rent = ctx.svm.minimum_balance_for_rent_exemption(Mint::LEN);

        let ixs = [
            system_instruction::create_account(
                &payer,
                &mint,
                rent,
                Mint::LEN as u64,
                &spl_token::ID,
            ),
            spl_token::instruction::initialize_mint(&spl_token::ID, &mint, &payer, None, decimals)
                .expect("initialize_mint failed"),
        ];

        let payer = ctx.payer.insecure_clone();
        ctx.send_tx(&ixs[..], Some(&[&payer, &mint_kp]))?;

        Ok((mint, mint_kp))
    }

    pub fn create_token_account(
        ctx: &mut SimContext,
        mint: &Pubkey,
        owner: Pubkey,
        initial_amount: u64,
    ) -> ResultSimulation<Pubkey> {
        let token_account = get_associated_token_address(&owner, mint);

        // Create ATA
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &ctx.payer.pubkey(), 
                &owner,
                mint,
                &spl_token::ID,
            );

        ctx.send_tx(&[create_ata_ix], None)?;

        // Mint tokens to the account if amount > 0
        if initial_amount > 0 {
            let mint_to_ix = spl_token::instruction::mint_to(
                &spl_token::ID,
                mint,
                &token_account,
                &ctx.payer.pubkey(),
                &[],
                initial_amount,
            )?;

            ctx.send_tx(&[mint_to_ix], None)?;
        }

        Ok(token_account)
    }

    /// Adjust token balance (for simulation purposes - rebalancing on external market)
    /// Positive delta = add tokens, Negative delta = remove tokens
    /// This directly modifies the token account balance in the simulation
    pub fn adjust_token_balance(
        ctx: &mut SimContext,
        token_account: Pubkey,
        delta: i64,
    ) -> ResultSimulation<()> {
        use spl_token::state::Account as TokenAccount;

        // Get current token account
        let mut account = ctx.svm.get_account(&token_account)
            .ok_or_else(|| crate::error::SimulationError::SystemError("Token account not found".into()))?;
        
        // Unpack token account data
        let mut token_data = TokenAccount::unpack(&account.data)
            .map_err(|e| crate::error::SimulationError::SystemError(format!("Failed to unpack token account: {:?}", e)))?;
        
        // Adjust balance
        let new_balance = if delta >= 0 {
            token_data.amount.checked_add(delta as u64)
        } else {
            token_data.amount.checked_sub((-delta) as u64)
        }.ok_or_else(|| crate::error::SimulationError::SystemError("Balance adjustment overflow/underflow".into()))?;

        token_data.amount = new_balance;

        // Pack back
        TokenAccount::pack(token_data, &mut account.data)
            .map_err(|e| crate::error::SimulationError::SystemError(format!("Failed to pack token account: {:?}", e)))?;

        // Update account in SVM
        ctx.svm.set_account(token_account, account.into())
            .map_err(|e| crate::error::SimulationError::SystemError(format!("Failed to set account: {:?}", e)))?;

        Ok(())
    }
}

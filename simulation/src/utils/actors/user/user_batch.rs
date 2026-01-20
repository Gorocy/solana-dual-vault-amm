use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{SeedDerivable, Signer},
};

use crate::utils::{
    actors::account::SimUser,
    args::SEED_OFFSET_USER_BATCH,
    ix::{context::SimContext, token_utils::TokenUtils},
};
use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};

#[derive(Debug)]
pub struct UserStats {
    pub user_index: usize,
    pub pubkey: Pubkey,
    pub balance_a: u64,
    pub balance_b: u64,
}

pub fn create_sim_users(
    ctx: &mut SimContext,
    mint_a: Pubkey,
    mint_b: Pubkey,
    user_count: usize,
    balance_a_range: std::ops::RangeInclusive<u64>,
    balance_b_range: std::ops::RangeInclusive<u64>,
    seed: u64,
) -> Vec<SimUser> {
    let mut rng = StdRng::seed_from_u64(seed.wrapping_add(SEED_OFFSET_USER_BATCH));

    let mut users = Vec::with_capacity(user_count);

    for i in 0..user_count {
        let mut seed_bytes = [0u8; 32];
        rng.fill_bytes(&mut seed_bytes);

        seed_bytes[0..8].copy_from_slice(&(i as u64).to_le_bytes());

        let user = Keypair::from_seed(&seed_bytes).expect("Failed to create keypair from seed");

        ctx.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        // ─────────────────────────────────────────────
        let balance_a = rng.random_range(balance_a_range.clone());
        let balance_b = rng.random_range(balance_b_range.clone());

        // ─────────────────────────────────────────────
        let token_a =
            TokenUtils::create_token_account(ctx, &mint_a, user.pubkey(), balance_a).unwrap();

        let token_b =
            TokenUtils::create_token_account(ctx, &mint_b, user.pubkey(), balance_b).unwrap();

        users.push(SimUser {
            keypair: user,
            token_a,
            token_b,
        });
    }

    users
}

/// Helper do tworzenia zrównoważonych użytkowników dla testów
pub fn create_balanced_sim_users(
    ctx: &mut SimContext,
    mint_a: Pubkey,
    mint_b: Pubkey,
    user_count: usize,
    balance_per_token: u64,
    seed: u64,
) -> Vec<SimUser> {
    create_sim_users(
        ctx,
        mint_a,
        mint_b,
        user_count,
        balance_per_token..=balance_per_token,
        balance_per_token..=balance_per_token,
        seed,
    )
}

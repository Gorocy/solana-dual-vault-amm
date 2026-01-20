#[cfg(test)]
mod tests {
    use crate::utils::{
        actors::user::user_batch::create_sim_users,
        ix::{context::SimContext, token_utils::TokenUtils},
    };
    use solana_sdk::signature::Signer;

    #[test]
    fn test_create_sim_users_deterministic_and_valid() {
        let mut ctx = SimContext::new();

        ctx.airdrop_payer(20_000_000_000).unwrap();

        // Create test mints
        let mint_a = TokenUtils::create_mint(&mut ctx, 6).unwrap();
        let mint_b = TokenUtils::create_mint(&mut ctx, 6).unwrap();

        let user_count = 5;
        let balance_a = 1_000_000;
        let balance_b = 2_000_000;
        let seed = 42;

        let users = create_sim_users(
            &mut ctx,
            mint_a.0,
            mint_b.0,
            user_count,
            balance_a / 4..=balance_a,
            balance_b / 4..=balance_b,
            seed,
        );

        ctx.next_slot();

        assert_eq!(users.len(), user_count);

        // Pubkeys must be unique
        let mut pubkeys = users.iter().map(|u| u.keypair.pubkey()).collect::<Vec<_>>();
        pubkeys.sort();
        pubkeys.dedup();
        assert_eq!(pubkeys.len(), user_count);

        // ─────────────────────────────────────────────
        for user in &users {
            // Key must be on-curve
            assert!(
                user.keypair.pubkey().is_on_curve(),
                "User pubkey must be on-curve"
            );

            // Token A balance (z SVM)
            let bal_a = user.get_balance_a(&ctx.svm);
            assert!(bal_a > 0);

            // Token B balance (z SVM)
            let bal_b = user.get_balance_b(&ctx.svm);
            assert!(bal_b > 0);

            // SOL balance should be > 0 (airdrop worked)
            let sol_balance = ctx
                .svm
                .get_account(&user.keypair.pubkey())
                .unwrap()
                .lamports;
            assert!(sol_balance > 0);
        }

        // ─────────────────────────────────────────────
        let users_again = create_sim_users(
            &mut ctx,
            mint_a.0,
            mint_b.0,
            user_count,
            balance_a / 4..=balance_a,
            balance_b / 4..=balance_b,
            seed,
        );

        for (u1, u2) in users.iter().zip(users_again.iter()) {
            assert_eq!(
                u1.keypair.pubkey(),
                u2.keypair.pubkey(),
                "Users must be deterministic for the same seed"
            );
        }
    }
}

use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Signer};

use crate::utils::ix::{
    context::SimContext,
    registry_utils::RegistryUtils,
    token_utils::{TokenPair, TokenUtils},
    FeeOption,
};

/// Podstawowy setup środowiska testowego z deployem programu i airdropem
pub fn setup_basic_env(ctx: &mut SimContext) {
    ctx.deploy_program("cpmm_rebalansing", crate::PROGRAM_ID);
    ctx.airdrop_payer(10 * LAMPORTS_PER_SOL).unwrap();
}

/// Setup z sortowanymi mintami (korzysta z TokenUtils::setup_sorted_mints)
pub fn setup_env_with_tokens(ctx: &mut SimContext) -> TokenPair {
    setup_basic_env(ctx);

    TokenUtils::setup_sorted_mints(ctx)
}

/// Pełny setup: środowisko + tokeny + registry
/// Zwraca: (Registry PDA, Mint A, Mint B)
pub fn setup_env_with_registry(ctx: &mut SimContext) -> (Pubkey, Pubkey, Pubkey) {
    let token_pair = setup_env_with_tokens(ctx);

    let registry_pda = RegistryUtils::initialize_vault_registry(
        ctx,
        token_pair.mint_a,
        token_pair.mint_b,
        FeeOption::Tier30,
    )
    .expect("Failed to init registry in setup");

    (registry_pda, token_pair.mint_a, token_pair.mint_b)
}

/// Setup z różnymi opcjami fee dla registry
pub fn setup_env_with_registry_fee(
    ctx: &mut SimContext,
    fee: FeeOption,
) -> (Pubkey, Pubkey, Pubkey) {
    let token_pair = setup_env_with_tokens(ctx);

    let registry_pda =
        RegistryUtils::initialize_vault_registry(ctx, token_pair.mint_a, token_pair.mint_b, fee)
            .expect("Failed to init registry in setup");

    (registry_pda, token_pair.mint_a, token_pair.mint_b)
}

/// Setup z wieloma parami tokenów
/// Zwraca: Vec<(Registry PDA, Mint A, Mint B)>
pub fn setup_multiple_token_pairs(
    ctx: &mut SimContext,
    count: usize,
) -> Vec<(Pubkey, Pubkey, Pubkey)> {
    setup_basic_env(ctx);

    let mut pairs = Vec::new();

    for _ in 0..count {
        let token_pair = TokenUtils::setup_sorted_mints(ctx);
        let registry_pda = RegistryUtils::initialize_vault_registry(
            ctx,
            token_pair.mint_a,
            token_pair.mint_b,
            FeeOption::Tier30,
        )
        .expect("Failed to init registry in setup");

        pairs.push((registry_pda, token_pair.mint_a, token_pair.mint_b));
    }

    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_env_setup() {
        let mut ctx = SimContext::new();

        setup_basic_env(&mut ctx);

        // Sprawdź czy payer ma środki
        let payer_account = ctx.svm.get_account(&ctx.payer.pubkey());
        assert!(payer_account.is_some());
        assert!(payer_account.unwrap().lamports > 0);
    }

    #[test]
    fn test_env_with_tokens_setup() {
        let mut ctx = SimContext::new();

        let token_pair = setup_env_with_tokens(&mut ctx);

        // Sprawdź czy minty są posortowane
        assert!(token_pair.mint_a > token_pair.mint_b);

        // Sprawdź czy minty istnieją
        assert!(ctx.svm.get_account(&token_pair.mint_a).is_some());
        assert!(ctx.svm.get_account(&token_pair.mint_b).is_some());
    }

    #[test]
    fn test_env_with_registry_setup() {
        let mut ctx = SimContext::new();

        let (registry, mint_a, mint_b) = setup_env_with_registry(&mut ctx);

        // Sprawdź czy registry istnieje
        assert!(ctx.svm.get_account(&registry).is_some());

        // Sprawdź sortowanie
        assert!(mint_a > mint_b);
    }
}

use anchor_lang::prelude::*;

#[constant]
pub const VAULT_REGISTRY_SEED: &[u8] = b"vault_registry";

#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

#[constant]
pub const LP_TOKEN_SEED: &[u8] = b"lp_token";

#[constant]
pub const DECIMALS: u8 = 9;

#[constant]
pub const FEE_DENOMINATOR: u64 = 10_000;

/// Minimum liquidity that must remain in vault after swap to prevent dust attacks
#[constant]
pub const MINIMUM_LIQUIDITY: u64 = 1_000;

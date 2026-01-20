use anchor_lang::{prelude::Pubkey, require, Result};
use anchor_spl::associated_token::spl_associated_token_account::solana_program::log::sol_log_compute_units;

use crate::{VaultRotate, vault::get_round_robin_pair};

/// Determines a deterministic vault pair for a user in a given slot.
/// Uses round-robin tournament scheduling to ensure that:
/// - each vault participates in exactly one pair per slot
/// - pairs rotate deterministically over time
///
/// For N=4 vaults:
/// Slot 0: (0,1), (2,3)
/// Slot 1: (0,2), (1,3)
/// Slot 2: (0,3), (1,2)
pub fn get_vault_pair(
    user_pubkey: &Pubkey,
    current_slot: u64,
    vault_count: u8,
) -> Result<(u8, u8)> {
    require!(vault_count >= 2, VaultRotate::InvalidVaultCount);

    // If vault count is odd, one vault is unused in each round
    let active_vaults = if vault_count % 2 == 0 {
        vault_count
    } else {
        vault_count - 1
    };

    let pairs_per_slot = active_vaults / 2;

    // Deterministic user-based sharding into pair slots
    sol_log_compute_units();
    let user_hash = user_hash(user_pubkey);
    sol_log_compute_units();
    let pair_index = (user_hash % pairs_per_slot as u64) as u8;

    // Round-robin rotation index
    let round = (current_slot % (active_vaults as u64 - 1)) as u8;

    Ok(get_round_robin_pair(pair_index, round, active_vaults))
}

/// Returns all valid vault pairs for a given slot using round-robin scheduling
pub fn get_vault_pairs_for_slot(
    current_slot: u64,
    vault_count: u8,
) -> Result<Vec<(u8, u8)>> {
    require!(vault_count >= 2, VaultRotate::InvalidVaultCount);

    let active_vaults = if vault_count % 2 == 0 {
        vault_count
    } else {
        vault_count - 1
    };

    let round = (current_slot % (active_vaults as u64 - 1)) as u8;
    let pairs_per_slot = active_vaults / 2;

    let mut pairs = Vec::with_capacity(pairs_per_slot as usize);

    for pair_index in 0..pairs_per_slot {
        let (a, b) = get_round_robin_pair(pair_index, round, active_vaults);
        pairs.push((a, b));
    }

    Ok(pairs)
}



/// Lightweight deterministic hash of a Pubkey.
/// This is NOT a cryptographic hash.
///
/// Purpose:
/// - user sharding
/// - deterministic pairing
/// - low compute cost
#[cfg(feature = "hash")]
#[inline(always)]
fn hash_pubkey(pubkey: &Pubkey) -> u64 {
    let bytes = pubkey.to_bytes();

    let mut hash = 0u64;
    for chunk in bytes.chunks(8) {
        let mut val = 0u64;
        for (i, &b) in chunk.iter().enumerate() {
            val |= (b as u64) << (i * 8);
        }
        // Golden ratio multiplier for good bit diffusion
        hash = hash.wrapping_add(val).wrapping_mul(0x9e3779b97f4a7c15);
    }

    hash
}

#[cfg(not(feature = "hash"))]
#[inline(always)]
fn get_last_bytes_as_u64(pubkey: &Pubkey) -> u64 {
    let bytes = pubkey.to_bytes();
    let arr: [u8; 8] = bytes[24..32].try_into().expect("slice is always 8 bytes");
    u64::from_le_bytes(arr)
}

#[inline(always)]
fn user_hash(pubkey: &Pubkey) -> u64 {
    #[cfg(feature = "hash")]
    {
        hash_pubkey(pubkey)
    }

    #[cfg(not(feature = "hash"))]
    {
        get_last_bytes_as_u64(pubkey)
    }
}

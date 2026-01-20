use anchor_lang::{require, Result};

use crate::VaultRotate;

/// Validates if a vault pair is valid for a given slot without allocating.
/// More efficient than get_vault_pairs_for_slot when checking a specific pair.
pub fn is_valid_vault_pair_for_slot(
    vault1: u8,
    vault2: u8,
    current_slot: u64,
    vault_count: u8,
) -> Result<bool> {
    require!(vault_count >= 2, VaultRotate::InvalidVaultCount);

    if vault1 == vault2 {
        return Ok(false);
    }

    let active_vaults = if vault_count % 2 == 0 {
        vault_count
    } else {
        vault_count - 1
    };

    // If one of the vaults is the inactive vault (when odd count)
    if vault1 >= active_vaults || vault2 >= active_vaults {
        return Ok(false);
    }

    let round = (current_slot % (active_vaults as u64 - 1)) as u8;
    let pairs_per_slot = active_vaults / 2;

    // Check all pairs for this slot
    for pair_index in 0..pairs_per_slot {
        let (a, b) = get_round_robin_pair(pair_index, round, active_vaults);
        if (a == vault1 && b == vault2) || (a == vault2 && b == vault1) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Generates a vault pair using round-robin tournament scheduling.
///
/// Properties:
/// - Every vault appears exactly once per round
/// - All possible pairings are generated across rounds
/// - Vault 0 is used as a fixed pivot
#[inline(always)]
pub(super) fn get_round_robin_pair(pair_index: u8, round: u8, n: u8) -> (u8, u8) {
    // First pair always includes the pivot vault (0)
    if pair_index == 0 {
        let opponent = 1 + (round % (n - 1));
        return (0, opponent);
    }

    // Remaining pairs rotate around the pivot
    let offset = pair_index * 2;

    let pos1 = 1 + ((round + offset - 1) % (n - 1));
    let pos2 = 1 + ((round + offset) % (n - 1));

    // Return ordered pair for consistency
    if pos1 < pos2 {
        (pos1, pos2)
    } else {
        (pos2, pos1)
    }
}

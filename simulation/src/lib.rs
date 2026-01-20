pub mod error;
pub mod output;
pub mod scenarios;
#[cfg(test)]
pub mod tests;
pub mod utils;

pub use cpmm_rebalansing::constants::*;

pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(cpmm_rebalansing::ID.to_bytes());

use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;

pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", name));
    let hash = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

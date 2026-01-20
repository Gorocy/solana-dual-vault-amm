use anchor_lang::error_code;
#[cfg(feature = "lineup")]
pub mod lineup;
pub mod vault;

#[cfg(feature = "lineup")]
pub use lineup::*;
pub use vault::*;

#[error_code]
pub enum VaultRotate {
    #[msg("Less than two vaults available for pairing")]
    InvalidVaultCount,
}

use anchor_lang::error_code;

pub mod dual_ix;
#[cfg(feature = "test_single")]
pub mod single_ix;

pub use dual_ix::*;
#[cfg(feature = "test_single")]
pub use single_ix::*;

#[error_code]
pub enum ErrorSwap {
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,

    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,

    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Invalid vault")]
    InvalidVault,

    #[msg("Liquidity too low - minimum liquidity constraint violated")]
    LiquidityTooLow,

    #[msg("Invalid fee calculation")]
    InvalidFeeCalculation,

    #[msg("Price impact too high")]
    PriceImpactTooHigh,
}

pub mod initialize;
pub mod liquidity;
pub mod swap;

use anchor_lang::error_code;
pub use initialize::*;
pub use liquidity::*;
pub use swap::*;

#[error_code]
pub enum InstructionExecutionError {
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
}

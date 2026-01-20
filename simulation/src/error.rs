use litesvm::types::FailedTransactionMetadata;
use thiserror::Error;

pub type ResultSimulation<T> = std::result::Result<T, SimulationError>;

#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("Failed transaction: {0:?}")]
    FailedTransactionMetadata(FailedTransactionMetadata),
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error("System error: {0}")]
    SystemError(String),
    #[error(transparent)]
    ProgramError(#[from] solana_program::program_error::ProgramError),
    #[error(transparent)]
    LiteSVMError(#[from] litesvm::error::LiteSVMError),
}

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum ProxyError {
    #[error("Program not initialized")]
    NotInitialized,
    
    #[error("Program already initialized")]
    AlreadyInitialized,
    
    #[error("Invalid authority")]
    InvalidAuthority,
    
    #[error("Invalid multiplier: must be greater than 0")]
    InvalidMultiplier,
    
    #[error("Invalid mint account")]
    InvalidMint,
    
    #[error("Invalid state account")]
    InvalidStateAccount,
    
    #[error("Invalid PDA derivation")]
    InvalidPDA,
}

impl From<ProxyError> for ProgramError {
    fn from(e: ProxyError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
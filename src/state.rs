use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Program state stored in a PDA account
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ProxyState {
    /// Whether the program has been initialized
    pub initialized: bool,
    /// Current authority of the proxy program
    pub authority: Pubkey,
    /// The token mint this proxy controls
    pub token_mint: Pubkey,
    /// PDA bump seed for the authority PDA
    pub bump: u8,
}

impl ProxyState {
    pub const LEN: usize = 1 + 32 + 32 + 1; // bool + 2 Pubkeys + u8

    /// Authority PDA seeds
    pub const AUTHORITY_SEED: &'static [u8] = b"proxy_authority";
    
    /// State PDA seeds  
    pub const STATE_SEED: &'static [u8] = b"state";
}
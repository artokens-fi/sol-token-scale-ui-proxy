use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum ProxyInstruction {
    /// Initialize the proxy program
    ///
    /// Accounts:
    /// 0. `[writable, signer]` Payer
    /// 1. `[writable]` State account (PDA)
    /// 2. `[]` Authority PDA
    /// 3. `[]` Token mint
    /// 4. `[]` System program
    Initialize {
        /// Initial authority for the proxy
        authority: Pubkey,
    },

    /// Update the token multiplier via CPI to Token-2022
    ///
    /// Accounts:
    /// 0. `[signer]` Current authority
    /// 1. `[writable]` State account (PDA)
    /// 2. `[]` Authority PDA
    /// 3. `[writable]` Token mint
    /// 4. `[]` Token-2022 program
    UpdateMultiplier {
        /// New multiplier (must be > 1.0)
        new_multiplier: f64,
        /// Unix timestamp when new multiplier becomes effective
        effective_timestamp: i64,
    },

    /// Update the program authority
    ///
    /// Accounts:
    /// 0. `[signer]` Current authority
    /// 1. `[writable]` State account (PDA)
    UpdateAuthority {
        /// New authority
        new_authority: Pubkey,
    },
}
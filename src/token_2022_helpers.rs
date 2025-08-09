use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use borsh::{BorshSerialize, BorshDeserialize};

/// Token-2022 Program ID
pub const TOKEN_2022_PROGRAM_ID: Pubkey = solana_program::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Token instruction types (from Token-2022 source)
#[derive(Clone, Copy, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum TokenInstruction {
    /// Extension instruction - discriminant: 43
    ScaledUiAmountExtension = 43,
}

/// Scaled UI Amount extension instruction types
#[derive(Clone, Copy, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum ScaledUiAmountMintInstruction {
    /// Initialize a new mint with scaled UI amounts
    Initialize = 0,
    /// Update the multiplier
    UpdateMultiplier = 1,
}

/// Create an UpdateMultiplier instruction for Token-2022 Scaled UI Amount extension
pub fn update_multiplier(
    mint: &Pubkey,
    authority: &Pubkey,
    multiplier: f64,
    effective_timestamp: i64,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    // Create the full instruction data manually
    // Format: [main_instruction_type, sub_instruction_type, multiplier_bytes, timestamp_bytes]
    let mut instruction_data = vec![];
    instruction_data.push(TokenInstruction::ScaledUiAmountExtension as u8);
    instruction_data.push(ScaledUiAmountMintInstruction::UpdateMultiplier as u8);
    instruction_data.extend_from_slice(&multiplier.to_le_bytes());
    instruction_data.extend_from_slice(&effective_timestamp.to_le_bytes());

    Ok(Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts,
        data: instruction_data,
    })
}
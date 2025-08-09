use crate::{error::ProxyError, instruction::ProxyInstruction, state::ProxyState, token_2022_helpers};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = ProxyInstruction::try_from_slice(instruction_data)?;
        match instruction {
            ProxyInstruction::Initialize { authority } => {
                Self::process_initialize(program_id, accounts, authority)
            }
            ProxyInstruction::UpdateMultiplier {
                new_multiplier,
                effective_timestamp,
            } => Self::process_update_multiplier(
                program_id,
                accounts,
                new_multiplier,
                effective_timestamp,
            ),
            ProxyInstruction::UpdateAuthority { new_authority } => {
                Self::process_update_authority(program_id, accounts, new_authority)
            }
        }
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        authority: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let payer_info = next_account_info(account_info_iter)?;
        let state_info = next_account_info(account_info_iter)?;
        let authority_pda_info = next_account_info(account_info_iter)?;
        let token_mint_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        // Verify payer is signer
        if !payer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Derive and verify state PDA
        let (state_pda, state_bump) = Pubkey::find_program_address(
            &[ProxyState::STATE_SEED],
            program_id,
        );
        if state_pda != *state_info.key {
            return Err(ProxyError::InvalidStateAccount.into());
        }

        // Derive and verify authority PDA
        let (authority_pda, authority_bump) = Pubkey::find_program_address(
            &[ProxyState::AUTHORITY_SEED],
            program_id,
        );
        if authority_pda != *authority_pda_info.key {
            return Err(ProxyError::InvalidPDA.into());
        }

        // Check if already initialized
        if state_info.data_len() > 0 {
            let state = ProxyState::try_from_slice(&state_info.data.borrow())?;
            if state.initialized {
                return Err(ProxyError::AlreadyInitialized.into());
            }
        }

        // Create state account
        let rent = Rent::get()?;
        let space = ProxyState::LEN;
        let lamports = rent.minimum_balance(space);

        let create_account_ix = system_instruction::create_account(
            payer_info.key,
            state_info.key,
            lamports,
            space as u64,
            program_id,
        );

        invoke_signed(
            &create_account_ix,
            &[payer_info.clone(), state_info.clone(), system_program_info.clone()],
            &[&[ProxyState::STATE_SEED, &[state_bump]]],
        )?;

        // Initialize state
        let state = ProxyState {
            initialized: true,
            authority,
            token_mint: *token_mint_info.key,
            bump: authority_bump,
        };

        state.serialize(&mut *state_info.data.borrow_mut())?;

        msg!("Proxy initialized with authority: {}", authority);
        msg!("Authority PDA: {}", authority_pda);
        msg!("Token mint: {}", token_mint_info.key);

        Ok(())
    }

    fn process_update_multiplier(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_multiplier: f64,
        effective_timestamp: i64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let authority_info = next_account_info(account_info_iter)?;
        let state_info = next_account_info(account_info_iter)?;
        let authority_pda_info = next_account_info(account_info_iter)?;
        let token_mint_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        // Verify authority is signer
        if !authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Load and verify state
        let state = ProxyState::try_from_slice(&state_info.data.borrow())?;
        if !state.initialized {
            return Err(ProxyError::NotInitialized.into());
        }

        // Verify authority
        if state.authority != *authority_info.key {
            return Err(ProxyError::InvalidAuthority.into());
        }

        // Verify token mint matches
        if state.token_mint != *token_mint_info.key {
            return Err(ProxyError::InvalidMint.into());
        }

        // Verify multiplier
        try_validate_multiplier(new_multiplier)?;

        // Verify authority PDA
        let (authority_pda, authority_bump) = Pubkey::find_program_address(
            &[ProxyState::AUTHORITY_SEED],
            program_id,
        );
        if authority_pda != *authority_pda_info.key || authority_bump != state.bump {
            return Err(ProxyError::InvalidPDA.into());
        }

        // Create Token-2022 update multiplier instruction using our helper
        let update_ix = token_2022_helpers::update_multiplier(
            token_mint_info.key,
            authority_pda_info.key,
            new_multiplier,
            effective_timestamp,
        )?;

        // Invoke with PDA signer
        invoke_signed(
            &update_ix,
            &[token_mint_info.clone(), authority_pda_info.clone()],
            &[&[ProxyState::AUTHORITY_SEED, &[state.bump]]],
        )?;

        msg!("Updated multiplier to {} effective at timestamp {}", new_multiplier, effective_timestamp);

        Ok(())
    }

    fn process_update_authority(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_authority: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let authority_info = next_account_info(account_info_iter)?;
        let state_info = next_account_info(account_info_iter)?;

        // Verify authority is signer
        if !authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Load and verify state
        let mut state = ProxyState::try_from_slice(&state_info.data.borrow())?;
        if !state.initialized {
            return Err(ProxyError::NotInitialized.into());
        }

        // Verify current authority
        if state.authority != *authority_info.key {
            return Err(ProxyError::InvalidAuthority.into());
        }

        // Verify new authority is not zero
        if new_authority == Pubkey::default() {
            return Err(ProxyError::InvalidAuthority.into());
        }

        // Update authority
        state.authority = new_authority;
        state.serialize(&mut *state_info.data.borrow_mut())?;

        msg!("Authority updated from {} to {}", authority_info.key, new_authority);

        Ok(())
    }
}

fn try_validate_multiplier(multiplier: f64) -> ProgramResult {
    if multiplier.is_sign_positive() && multiplier.is_normal() {
        Ok(())
    } else {
        Err(ProxyError::InvalidMultiplier.into())
    }
}
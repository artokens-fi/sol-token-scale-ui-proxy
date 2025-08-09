# Token-2022 Scaled UI Amount Proxy Program Design

## Problem Statement

The Token-2022 Scaled UI Amount extension has a significant limitation: once the authority is set during initialization, it cannot be changed. There is no `update_authority` instruction in the extension. This creates operational risks:

- If the authority keypair is compromised, there's no recovery mechanism
- If the authority needs to be transferred (e.g., DAO governance, company ownership change), it's impossible
- No ability to implement sophisticated access control (multi-sig, time-locks, etc.)

## Proposed Solution

Create a **Proxy Program** that acts as an intermediary authority for the Scaled UI Amount extension. This program will:

1. Be set as the authority for the Token-2022 Scaled UI Amount extension
2. Manage its own upgradeable authority
3. Forward multiplier update requests via Cross Program Invocation (CPI)
4. Support a SINGLE token mint (one program instance = one token)

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│         User/Client Application             │
└─────────────────┬───────────────────────────┘
                  │ Sends Transaction
                  ▼
┌─────────────────────────────────────────────┐
│         Proxy Program (Our Program)         │
│                                             │
│  State Account (PDA):                      │
│  - authority: Pubkey (updateable)          │
│  - token_mint: Pubkey                      │
│  - bump: u8                                 │
│                                             │
│  Instructions:                              │
│  - initialize()                             │
│  - update_multiplier()                      │
│  - update_authority()                       │
└─────────────────┬───────────────────────────┘
                  │ CPI with PDA Signer
                  ▼
┌─────────────────────────────────────────────┐
│      Token-2022 Program                     │
│                                             │
│  Token Mint with Scaled UI Amount Extension │
│  - authority: Proxy Program PDA             │
│  - multiplier: current value                │
│  - new_multiplier: scheduled value          │
└─────────────────────────────────────────────┘
```

## Key Design Decisions

### 1. Dual PDA Architecture

The program uses two separate PDAs:

- **Authority PDA**: Controls the Token-2022 mint's Scaled UI Amount extension
  - Seeds: `[b"proxy_authority"]` 
  - This PDA is set as the authority for the Token-2022 extension
  - Used for signing CPI calls to Token-2022

- **State PDA**: Stores the program's configuration data
  - Seeds: `[b"state"]`
  - Stores authority, mint, initialization status, and authority PDA bump

### 2. State Management

```rust
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ProxyState {
    pub initialized: bool,           // Ensure single initialization
    pub authority: Pubkey,           // Current authority of the proxy program
    pub token_mint: Pubkey,          // The SINGLE token mint this proxy controls
    pub bump: u8,                    // PDA bump seed for the authority PDA
}

impl ProxyState {
    pub const LEN: usize = 1 + 32 + 32 + 1; // bool + 2 Pubkeys + u8
    
    /// Authority PDA seeds
    pub const AUTHORITY_SEED: &'static [u8] = b"proxy_authority";
    
    /// State PDA seeds  
    pub const STATE_SEED: &'static [u8] = b"state";
}
```

### 3. Instruction Set

#### `initialize`

- Creates the proxy state account using state PDA (seeds: `[b"state"]`)
- Verifies the authority PDA (seeds: `[b"proxy_authority"]`) 
- Sets initial authority
- Stores token mint reference
- **Note**: The Token-2022 mint must already exist with the proxy authority PDA as authority

**Accounts:**
- `[writable, signer]` Payer
- `[writable]` State account (PDA with seeds `[b"state"]`)
- `[]` Authority PDA (seeds `[b"proxy_authority"]`)
- `[]` Token mint
- `[]` System program

#### `update_multiplier`

- Validates caller is current authority
- Validates new multiplier is positive and normal (not NaN, infinity, or zero)
- Validates token mint matches stored mint
- Accepts new multiplier and effective timestamp
- Makes CPI to Token-2022 `ScaledUiAmountExtension::UpdateMultiplier` instruction
- Signs with proxy authority PDA

**Accounts:**
- `[signer]` Current authority
- `[writable]` State account (PDA)
- `[]` Authority PDA
- `[writable]` Token mint
- `[]` Token-2022 program

#### `update_authority`

- Validates caller is current authority
- Validates new authority is not zero address (Pubkey::default())
- Updates the authority in proxy state
- Immediate transfer (no two-step process)

**Accounts:**
- `[signer]` Current authority
- `[writable]` State account (PDA)

## Implementation Flow

### Initial Setup (One-time)

1. Deploy the proxy program (immutable)
2. Derive the proxy authority PDA address (seeds: `[b"proxy_authority"]`)
3. Create Token-2022 mint with Scaled UI Amount extension
   - Set proxy authority PDA as the extension authority
4. Initialize proxy program state by calling `initialize` instruction
   - Creates state PDA (seeds: `[b"state"]`)
   - Sets your wallet as initial authority
   - Stores mint address in proxy state

### Updating Multiplier

1. User calls proxy program's `update_multiplier`
2. Proxy validates current authority
3. Proxy validates multiplier is positive and normal
4. Proxy validates token mint matches stored mint
5. Proxy creates Token-2022 `ScaledUiAmountExtension::UpdateMultiplier` instruction
6. Proxy CPIs to Token-2022 with authority PDA signature
7. Token-2022 updates the multiplier

### Transferring Authority

1. Current authority calls `update_authority` with new authority
2. Authority is immediately transferred
3. Future multiplier updates require new authority

## Security Considerations

### Implemented Security Features

- Authority validation on all state-changing instructions
- Proper PDA derivation and validation
- Comprehensive error handling
- Prevention of authority loss (zero address check)
- Multiplier validation (must be positive and normal)
- Single initialization check (prevent re-initialization)
- Program immutability (no upgrade authority)

## Technical Implementation Details

### CPI to Token-2022

The implementation uses a custom Token-2022 helper module since the standard SPL crate doesn't expose the scaled UI amount extension instructions:

```rust
// From token_2022_helpers.rs
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

    // Create instruction data manually
    let mut instruction_data = vec![];
    instruction_data.push(TokenInstruction::ScaledUiAmountExtension as u8); // 43
    instruction_data.push(ScaledUiAmountMintInstruction::UpdateMultiplier as u8); // 1
    instruction_data.extend_from_slice(&multiplier.to_le_bytes());
    instruction_data.extend_from_slice(&effective_timestamp.to_le_bytes());

    Ok(Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts,
        data: instruction_data,
    })
}

// In processor.rs - multiplier validation
fn try_validate_multiplier(multiplier: f64) -> ProgramResult {
    if multiplier.is_sign_positive() && multiplier.is_normal() {
        Ok(())
    } else {
        Err(ProxyError::InvalidMultiplier.into())
    }
}

// CPI invocation
invoke_signed(
    &update_ix,
    &[token_mint_info.clone(), authority_pda_info.clone()],
    &[&[ProxyState::AUTHORITY_SEED, &[state.bump]]],
)?;
```

### Error Cases to Handle

The implementation includes comprehensive error handling through the `ProxyError` enum:

1. **NotInitialized** - Program state not initialized
2. **AlreadyInitialized** - Prevent double initialization  
3. **InvalidAuthority** - Wrong signer or zero address for new authority
4. **InvalidMultiplier** - Multiplier not positive and normal (catches NaN, infinity, zero, negative)
5. **InvalidMint** - Token mint doesn't match stored mint
6. **InvalidStateAccount** - Wrong state PDA provided
7. **InvalidPDA** - Wrong PDA derivation for authority PDA
8. Missing required signatures (handled by Solana runtime)
9. CPI failures (Token-2022 program errors bubble up)

## Testing Strategy

### Unit Tests

- Initialize proxy state
- Update multiplier with valid authority and valid multiplier
- Reject update with invalid authority
- Reject update with invalid multiplier (NaN, infinity, zero, negative)
- Reject update with wrong token mint
- Update authority successfully
- Reject authority update to zero address
- Reject double initialization

### Integration Tests

- Full flow with actual Token-2022 program
- Multiple multiplier updates
- Authority transfer and subsequent operations
- Edge cases (max values, zero values, etc.)

## Confirmed Requirements

1. **Authority Model**: Single wallet authority
2. **Two-Step Transfer**: No, immediate transfer
3. **Additional Features**: None for MVP
4. **Deployment**: TBD (suggest devnet first for testing)
5. **Upgrade Authority**: Immutable program
6. **Token Support**: Single token only
7. **Multiplier Constraint**: Must be positive and normal (not NaN, infinity, zero, or negative)

## Development Phases

### Phase 1: MVP (What we're building)

- Single token support with dual PDA architecture
- Authority management with immediate transfer
- Custom Token-2022 CPI helper for scaled UI amount extension
- Robust multiplier validation (positive and normal)
- Comprehensive error handling
- Immutable deployment ready

## Implementation Status

**COMPLETE** - All core functionality implemented and ready for deployment

### Implemented Features

- Dual PDA architecture (authority + state)
- Initialize instruction with comprehensive validation
- Update multiplier with Token-2022 CPI
- Update authority with immediate transfer
- Custom Token-2022 helper module
- Robust error handling (7 custom error types)
- Multiplier validation (positive, normal, not NaN/infinity)
- Authority validation (non-zero address)
- Single initialization protection
- Mint validation

### Ready for Next Steps

1. Implementation complete
2. Write comprehensive tests  
3. Deploy to devnet for testing
4. Security review
5. Mainnet deployment

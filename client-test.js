#!/usr/bin/env node

const {
    Connection,
    PublicKey,
    Keypair,
    Transaction,
    TransactionInstruction,
    SystemProgram,
    sendAndConfirmTransaction,
} = require('@solana/web3.js');
const fs = require('fs');

// Constants
const PROGRAM_ID = new PublicKey('Df5BJgxG1pka2TVXH15eKPcwXDpwbfS1h8LvvnRE8qZt');
const TOKEN_MINT = new PublicKey('ucYGmoTNh2trwA2T734LfRRczcLkn6YiVSc3DTsdPfE');
const TOKEN_2022_PROGRAM_ID = new PublicKey('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb');

// PDA seeds
const STATE_SEED = Buffer.from('state');
const AUTHORITY_SEED = Buffer.from('proxy_authority');

// Instruction enum
const ProxyInstruction = {
    Initialize: 0,
    UpdateMultiplier: 1,
    UpdateAuthority: 2,
};

// Helper functions to manually serialize instruction data
function serializeInitialize(authority) {
    const buffer = Buffer.alloc(33); // 1 byte instruction + 32 bytes pubkey
    buffer.writeUInt8(0, 0); // instruction discriminant
    authority.toBuffer().copy(buffer, 1); // copy pubkey
    return buffer;
}

function serializeUpdateMultiplier(multiplier, timestamp) {
    const buffer = Buffer.alloc(17); // 1 byte instruction + 8 bytes f64 + 8 bytes i64
    buffer.writeUInt8(1, 0); // instruction discriminant
    buffer.writeDoubleLE(multiplier, 1); // f64 multiplier
    buffer.writeBigInt64LE(BigInt(timestamp), 9); // i64 timestamp
    return buffer;
}

function serializeUpdateAuthority(newAuthority) {
    const buffer = Buffer.alloc(33); // 1 byte instruction + 32 bytes pubkey
    buffer.writeUInt8(2, 0); // instruction discriminant
    newAuthority.toBuffer().copy(buffer, 1); // copy pubkey
    return buffer;
}

async function main() {
    // Connect to devnet
    const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
    
    // Load payer keypair (assuming you have a keypair file)
    let payer;
    try {
        const keypairData = JSON.parse(fs.readFileSync(process.env.HOME + '/Private/Auro/solana-scaled-ui-proxy/token_scale_ui_proxy/test-uiscale-wallet.json', 'utf8'));
        payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
    } catch (error) {
        console.error('Failed to load keypair. Make sure you have a Solana keypair configured.');
        process.exit(1);
    }

    console.log('Using payer:', payer.publicKey.toString());
    console.log('Program ID:', PROGRAM_ID.toString());
    console.log('Token Mint:', TOKEN_MINT.toString());

    // Calculate PDAs
    const [statePda] = PublicKey.findProgramAddressSync([STATE_SEED], PROGRAM_ID);
    const [authorityPda] = PublicKey.findProgramAddressSync([AUTHORITY_SEED], PROGRAM_ID);
    
    console.log('State PDA:', statePda.toString());
    console.log('Authority PDA:', authorityPda.toString());

    const command = process.argv[2];
    
    if (command === 'initialize') {
        await initializeProxy(connection, payer, statePda, authorityPda);
    } else if (command === 'update-multiplier') {
        const multiplier = parseFloat(process.argv[3]);
        const timestamp = parseInt(process.argv[4]);
        await updateMultiplier(connection, payer, statePda, authorityPda, multiplier, timestamp);
    } else if (command === 'update-authority') {
        const newAuthority = new PublicKey(process.argv[3]);
        await updateAuthority(connection, payer, statePda, newAuthority);
    } else {
        console.log('Usage:');
        console.log('  node client-test.js initialize');
        console.log('  node client-test.js update-multiplier [multiplier] [timestamp]');
        console.log('  node client-test.js update-authority <new_authority_pubkey>');
    }
}

async function initializeProxy(connection, payer, statePda, authorityPda) {
    console.log('\n=== Initializing Proxy ===');
    
    // Create instruction data
    const instructionData = serializeInitialize(payer.publicKey);
    
    // Create instruction
    const instruction = new TransactionInstruction({
        keys: [
            { pubkey: payer.publicKey, isSigner: true, isWritable: true },  // Payer
            { pubkey: statePda, isSigner: false, isWritable: true },        // State PDA
            { pubkey: authorityPda, isSigner: false, isWritable: false },   // Authority PDA
            { pubkey: TOKEN_MINT, isSigner: false, isWritable: false },     // Token mint
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // System program
        ],
        programId: PROGRAM_ID,
        data: Buffer.from(instructionData),
    });

    // Send transaction
    const transaction = new Transaction().add(instruction);
    
    try {
        const signature = await sendAndConfirmTransaction(connection, transaction, [payer]);
        console.log('Initialize successful!');
        console.log('Signature:', signature);
    } catch (error) {
        console.error('Initialize failed:', error);
        console.error('Logs:', error.logs);
    }
}

async function updateMultiplier(connection, payer, statePda, authorityPda, multiplier, timestamp) {
    console.log('\n=== Updating Multiplier ===');
    console.log('New multiplier:', multiplier);
    console.log('Effective timestamp:', timestamp);
    
    // Create instruction data
    const instructionData = serializeUpdateMultiplier(multiplier, timestamp);
    
    // Create instruction
    const instruction = new TransactionInstruction({
        keys: [
            { pubkey: payer.publicKey, isSigner: true, isWritable: false },  // Current authority
            { pubkey: statePda, isSigner: false, isWritable: true },         // State PDA
            { pubkey: authorityPda, isSigner: false, isWritable: false },    // Authority PDA  
            { pubkey: TOKEN_MINT, isSigner: false, isWritable: true },       // Token mint
            { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false }, // Token-2022 program
        ],
        programId: PROGRAM_ID,
        data: Buffer.from(instructionData),
    });

    // Send transaction
    const transaction = new Transaction().add(instruction);
    
    try {
        const signature = await sendAndConfirmTransaction(connection, transaction, [payer]);
        console.log('Update multiplier successful!');
        console.log('Signature:', signature);
    } catch (error) {
        console.error('Update multiplier failed:', error);
        console.error('Logs:', error.logs);
    }
}

async function updateAuthority(connection, payer, statePda, newAuthority) {
    console.log('\n=== Updating Authority ===');
    console.log('New authority:', newAuthority.toString());
    
    // Create instruction data
    const instructionData = serializeUpdateAuthority(newAuthority);
    
    // Create instruction
    const instruction = new TransactionInstruction({
        keys: [
            { pubkey: payer.publicKey, isSigner: true, isWritable: false },  // Current authority
            { pubkey: statePda, isSigner: false, isWritable: true },         // State PDA
        ],
        programId: PROGRAM_ID,
        data: Buffer.from(instructionData),
    });

    // Send transaction
    const transaction = new Transaction().add(instruction);
    
    try {
        const signature = await sendAndConfirmTransaction(connection, transaction, [payer]);
        console.log('Update authority successful!');
        console.log('Signature:', signature);
    } catch (error) {
        console.error('Update authority failed:', error);
        console.error('Logs:', error.logs);
    }
}

main().catch(console.error);
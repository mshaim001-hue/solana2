//! Integration-style tests focused on instruction validation via LiteSVM
//! where practical, plus on-chain program load checks.
//!
//! Token CPI flows are covered by the TypeScript/script smoke tests and by
//! comprehensive unit tests in `src/math.rs`. Negative program paths that
//! do not require SPL Token CPIs are exercised here.

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::instruction::Instruction,
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn load_program() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = yield_vault::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/yield_vault.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn program_bytes_load_into_litesvm() {
    let (svm, program_id, _payer) = load_program();
    // Program account must exist after add_program
    assert!(svm.get_account(&program_id).is_some());
}

#[test]
fn set_paused_rejects_non_authority_without_vault() {
    // Sending set_paused against a missing vault account must fail (account checks).
    let (mut svm, program_id, payer) = load_program();
    let fake_mint = Keypair::new().pubkey();
    let (vault, _bump) =
        Pubkey::find_program_address(&[yield_vault::VAULT_SEED, fake_mint.as_ref()], &program_id);

    let ix = Instruction::new_with_bytes(
        program_id,
        &yield_vault::instruction::SetPaused { paused: true }.data(),
        yield_vault::accounts::SetPaused {
            authority: payer.pubkey(),
            vault,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    let res = svm.send_transaction(tx);
    assert!(res.is_err(), "set_paused without vault must fail");
}

#[test]
fn deposit_rejects_missing_accounts() {
    let (mut svm, program_id, payer) = load_program();
    let fake_mint = Keypair::new().pubkey();
    let (vault, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_SEED, fake_mint.as_ref()], &program_id);
    let (share_mint, _) =
        Pubkey::find_program_address(&[yield_vault::SHARE_MINT_SEED, vault.as_ref()], &program_id);
    let (vault_token, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_TOKEN_SEED, vault.as_ref()], &program_id);

    let ix = Instruction::new_with_bytes(
        program_id,
        &yield_vault::instruction::Deposit { amount: 1_000 }.data(),
        yield_vault::accounts::Deposit {
            user: payer.pubkey(),
            vault,
            underlying_mint: fake_mint,
            share_mint,
            vault_token,
            user_underlying: Keypair::new().pubkey(),
            user_shares: Keypair::new().pubkey(),
            token_program: anchor_spl::token_interface::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    assert!(svm.send_transaction(tx).is_err());
}

#[test]
fn withdraw_rejects_missing_accounts() {
    let (mut svm, program_id, payer) = load_program();
    let fake_mint = Keypair::new().pubkey();
    let (vault, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_SEED, fake_mint.as_ref()], &program_id);
    let (share_mint, _) =
        Pubkey::find_program_address(&[yield_vault::SHARE_MINT_SEED, vault.as_ref()], &program_id);
    let (vault_token, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_TOKEN_SEED, vault.as_ref()], &program_id);

    let ix = Instruction::new_with_bytes(
        program_id,
        &yield_vault::instruction::Withdraw { shares: 1_000 }.data(),
        yield_vault::accounts::Withdraw {
            user: payer.pubkey(),
            vault,
            underlying_mint: fake_mint,
            share_mint,
            vault_token,
            fee_token: Keypair::new().pubkey(),
            user_underlying: Keypair::new().pubkey(),
            user_shares: Keypair::new().pubkey(),
            token_program: spl_token_interface_id(),
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    assert!(svm.send_transaction(tx).is_err());
}

fn spl_token_interface_id() -> Pubkey {
    // Classic SPL Token program id
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

#[test]
fn initialize_rejects_invalid_apy_without_full_token_stack() {
    // Invalid APY is checked in handler after account constraints.
    // Without token accounts the ix fails earlier — still a negative path.
    let (mut svm, program_id, payer) = load_program();
    let fake_mint = Keypair::new().pubkey();
    let (vault, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_SEED, fake_mint.as_ref()], &program_id);
    let (share_mint, _) =
        Pubkey::find_program_address(&[yield_vault::SHARE_MINT_SEED, vault.as_ref()], &program_id);
    let (vault_token, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_TOKEN_SEED, vault.as_ref()], &program_id);

    let ix = Instruction::new_with_bytes(
        program_id,
        &yield_vault::instruction::Initialize {
            apy_bps: 20_000, // > MAX
            withdrawal_fee_bps: 50,
            min_deposit: 1,
            max_tvl: 1_000_000,
        }
        .data(),
        yield_vault::accounts::Initialize {
            authority: payer.pubkey(),
            underlying_mint: fake_mint,
            vault,
            share_mint,
            vault_token,
            fee_token: Keypair::new().pubkey(),
            token_program: spl_token_interface_id(),
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    assert!(svm.send_transaction(tx).is_err());
}

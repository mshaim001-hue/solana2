//! LiteSVM integration tests for YieldVault.
//!
//! Requires a built program binary at `target/deploy/yield_vault.so`
//! (produced by `anchor build`). Happy-path coverage exercises real SPL Token
//! CPIs for initialize / deposit / withdraw / fund_rewards, plus authority and
//! min_out checks.

use {
    anchor_lang::{
        prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_account::Account,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program_option::COption,
    solana_program_pack::Pack,
    solana_rent::Rent,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    spl_token_interface::{
        state::{Account as TokenAccount, AccountState, Mint},
        ID as TOKEN_PROGRAM_ID,
    },
    std::path::PathBuf,
};

fn program_so_bytes() -> Vec<u8> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/yield_vault.so"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/deploy/yield_vault.so"),
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("../deploy/yield_vault.so"),
    ];
    for path in &candidates {
        if let Ok(bytes) = std::fs::read(path) {
            return bytes;
        }
    }
    panic!(
        "yield_vault.so not found. Run `anchor build` first. Looked in: {:?}",
        candidates
    );
}

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

fn pack_mint(authority: &Pubkey, decimals: u8, supply: u64) -> Vec<u8> {
    let mint = Mint {
        mint_authority: COption::Some(*authority),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    let mut data = vec![0u8; Mint::LEN];
    Mint::pack(mint, &mut data).unwrap();
    data
}

fn pack_token(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let acc = TokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(acc, &mut data).unwrap();
    data
}

fn rent_exempt(svm: &LiteSVM, size: usize) -> u64 {
    svm.get_sysvar::<Rent>().minimum_balance(size)
}

fn set_token_account(svm: &mut LiteSVM, key: Pubkey, mint: &Pubkey, owner: &Pubkey, amount: u64) {
    let data = pack_token(mint, owner, amount);
    let lamports = rent_exempt(svm, TokenAccount::LEN);
    svm.set_account(
        key,
        Account {
            lamports,
            data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn token_amount(svm: &LiteSVM, key: &Pubkey) -> u64 {
    let acc = svm.get_account(key).expect("token account missing");
    TokenAccount::unpack(&acc.data).unwrap().amount
}

fn send_ix(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx).map(|_| ()).map_err(|e| format!("{e:?}"))
}

struct VaultFixture {
    svm: LiteSVM,
    program_id: Pubkey,
    authority: Keypair,
    mint: Keypair,
    vault: Pubkey,
    share_mint: Pubkey,
    vault_token: Pubkey,
    authority_ata: Pubkey,
}

fn setup_initialized_vault() -> VaultFixture {
    let program_id = yield_vault::id();
    let authority = Keypair::new();
    let mint = Keypair::new();
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_so_bytes()).unwrap();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    // Underlying mint owned by authority
    let mint_data = pack_mint(&authority.pubkey(), 6, 0);
    svm.set_account(
        mint.pubkey(),
        Account {
            lamports: rent_exempt(&svm, Mint::LEN),
            data: mint_data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let (vault, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_SEED, mint.pubkey().as_ref()], &program_id);
    let (share_mint, _) =
        Pubkey::find_program_address(&[yield_vault::SHARE_MINT_SEED, vault.as_ref()], &program_id);
    let (vault_token, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_TOKEN_SEED, vault.as_ref()], &program_id);

    let authority_ata = Keypair::new().pubkey();
    set_token_account(
        &mut svm,
        authority_ata,
        &mint.pubkey(),
        &authority.pubkey(),
        1_000_000_000, // 1000 tokens @ 6 decimals
    );

    let init_ix = Instruction::new_with_bytes(
        program_id,
        &yield_vault::instruction::Initialize {
            apy_bps: 1_000,
            withdrawal_fee_bps: 50,
            min_deposit: 1_000_000,
            max_tvl: 1_000_000_000_000,
        }
        .data(),
        yield_vault::accounts::Initialize {
            authority: authority.pubkey(),
            underlying_mint: mint.pubkey(),
            vault,
            share_mint,
            vault_token,
            token_program: token_program_id(),
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
    );

    send_ix(&mut svm, &authority, init_ix).expect("initialize must succeed");

    VaultFixture {
        svm,
        program_id,
        authority,
        mint,
        vault,
        share_mint,
        vault_token,
        authority_ata,
    }
}

fn create_user_shares(fx: &mut VaultFixture, user: &Pubkey) -> Pubkey {
    let user_shares = Keypair::new().pubkey();
    set_token_account(&mut fx.svm, user_shares, &fx.share_mint, user, 0);
    user_shares
}

#[test]
fn program_bytes_load_into_litesvm() {
    let program_id = yield_vault::id();
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_so_bytes()).unwrap();
    assert!(svm.get_account(&program_id).is_some());
}

#[test]
fn initialize_deposit_withdraw_fund_rewards_happy_path() {
    let mut fx = setup_initialized_vault();
    let authority_pk = fx.authority.pubkey();
    let user_shares = create_user_shares(&mut fx, &authority_pk);

    let deposit_amt = 50_000_000u64; // 50 tokens
    let deposit_ix = Instruction::new_with_bytes(
        fx.program_id,
        &yield_vault::instruction::Deposit {
            amount: deposit_amt,
        }
        .data(),
        yield_vault::accounts::Deposit {
            user: fx.authority.pubkey(),
            vault: fx.vault,
            underlying_mint: fx.mint.pubkey(),
            share_mint: fx.share_mint,
            vault_token: fx.vault_token,
            user_underlying: fx.authority_ata,
            user_shares,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    send_ix(&mut fx.svm, &fx.authority, deposit_ix).expect("deposit");

    assert_eq!(token_amount(&fx.svm, &fx.vault_token), deposit_amt);
    assert_eq!(token_amount(&fx.svm, &user_shares), deposit_amt); // 1:1 first deposit
    assert_eq!(
        token_amount(&fx.svm, &fx.authority_ata),
        1_000_000_000 - deposit_amt
    );

    // Fund rewards: liquidity up, share supply unchanged
    let fund_amt = 10_000_000u64;
    let fund_ix = Instruction::new_with_bytes(
        fx.program_id,
        &yield_vault::instruction::FundRewards { amount: fund_amt }.data(),
        yield_vault::accounts::FundRewards {
            authority: fx.authority.pubkey(),
            vault: fx.vault,
            underlying_mint: fx.mint.pubkey(),
            vault_token: fx.vault_token,
            authority_underlying: fx.authority_ata,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    send_ix(&mut fx.svm, &fx.authority, fund_ix).expect("fund_rewards");
    assert_eq!(
        token_amount(&fx.svm, &fx.vault_token),
        deposit_amt + fund_amt
    );
    assert_eq!(token_amount(&fx.svm, &user_shares), deposit_amt);

    // Withdraw 10 shares @ 0.5% fee → net 9_950_000, fee 50_000 → same ATA (authority)
    let withdraw_shares = 10_000_000u64;
    let before_user = token_amount(&fx.svm, &fx.authority_ata);
    let withdraw_ix = Instruction::new_with_bytes(
        fx.program_id,
        &yield_vault::instruction::Withdraw {
            shares: withdraw_shares,
            min_out: 9_950_000,
        }
        .data(),
        yield_vault::accounts::Withdraw {
            user: fx.authority.pubkey(),
            vault: fx.vault,
            underlying_mint: fx.mint.pubkey(),
            share_mint: fx.share_mint,
            vault_token: fx.vault_token,
            fee_recipient_ata: fx.authority_ata,
            user_underlying: fx.authority_ata,
            user_shares,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    send_ix(&mut fx.svm, &fx.authority, withdraw_ix).expect("withdraw");

    // net + fee both credited to authority ATA when fee_recipient == user
    let after_user = token_amount(&fx.svm, &fx.authority_ata);
    assert_eq!(after_user - before_user, 10_000_000);
    assert_eq!(token_amount(&fx.svm, &user_shares), deposit_amt - withdraw_shares);
}

#[test]
fn fund_rewards_rejects_non_authority() {
    let mut fx = setup_initialized_vault();
    let impostor = Keypair::new();
    fx.svm.airdrop(&impostor.pubkey(), 1_000_000_000).unwrap();

    let impostor_ata = Keypair::new().pubkey();
    set_token_account(
        &mut fx.svm,
        impostor_ata,
        &fx.mint.pubkey(),
        &impostor.pubkey(),
        5_000_000,
    );

    let fund_ix = Instruction::new_with_bytes(
        fx.program_id,
        &yield_vault::instruction::FundRewards { amount: 1_000_000 }.data(),
        yield_vault::accounts::FundRewards {
            authority: impostor.pubkey(),
            vault: fx.vault,
            underlying_mint: fx.mint.pubkey(),
            vault_token: fx.vault_token,
            authority_underlying: impostor_ata,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    let err = send_ix(&mut fx.svm, &impostor, fund_ix).unwrap_err();
    assert!(
        err.contains("Unauthorized") || err.contains("custom program error") || err.contains("Error"),
        "expected unauthorized, got {err}"
    );
}

#[test]
fn set_paused_rejects_non_authority() {
    let mut fx = setup_initialized_vault();
    let impostor = Keypair::new();
    fx.svm.airdrop(&impostor.pubkey(), 1_000_000_000).unwrap();

    let ix = Instruction::new_with_bytes(
        fx.program_id,
        &yield_vault::instruction::SetPaused { paused: true }.data(),
        yield_vault::accounts::SetPaused {
            authority: impostor.pubkey(),
            vault: fx.vault,
        }
        .to_account_metas(None),
    );
    let err = send_ix(&mut fx.svm, &impostor, ix).unwrap_err();
    assert!(
        err.contains("Unauthorized") || err.contains("custom program error") || err.contains("Error"),
        "expected unauthorized, got {err}"
    );
}

#[test]
fn withdraw_rejects_min_out_slippage() {
    let mut fx = setup_initialized_vault();
    let authority_pk = fx.authority.pubkey();
    let user_shares = create_user_shares(&mut fx, &authority_pk);

    let deposit_ix = Instruction::new_with_bytes(
        fx.program_id,
        &yield_vault::instruction::Deposit {
            amount: 50_000_000,
        }
        .data(),
        yield_vault::accounts::Deposit {
            user: fx.authority.pubkey(),
            vault: fx.vault,
            underlying_mint: fx.mint.pubkey(),
            share_mint: fx.share_mint,
            vault_token: fx.vault_token,
            user_underlying: fx.authority_ata,
            user_shares,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    send_ix(&mut fx.svm, &fx.authority, deposit_ix).expect("deposit");

    // Request impossible min_out
    let withdraw_ix = Instruction::new_with_bytes(
        fx.program_id,
        &yield_vault::instruction::Withdraw {
            shares: 10_000_000,
            min_out: 9_999_999_999,
        }
        .data(),
        yield_vault::accounts::Withdraw {
            user: fx.authority.pubkey(),
            vault: fx.vault,
            underlying_mint: fx.mint.pubkey(),
            share_mint: fx.share_mint,
            vault_token: fx.vault_token,
            fee_recipient_ata: fx.authority_ata,
            user_underlying: fx.authority_ata,
            user_shares,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    let err = send_ix(&mut fx.svm, &fx.authority, withdraw_ix).unwrap_err();
    assert!(
        err.contains("Slippage") || err.contains("custom program error") || err.contains("Error"),
        "expected slippage error, got {err}"
    );
}

#[test]
fn deposit_rejects_missing_accounts() {
    let program_id = yield_vault::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_so_bytes()).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

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
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    assert!(send_ix(&mut svm, &payer, ix).is_err());
}

#[test]
fn withdraw_rejects_missing_accounts() {
    let program_id = yield_vault::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_so_bytes()).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let fake_mint = Keypair::new().pubkey();
    let (vault, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_SEED, fake_mint.as_ref()], &program_id);
    let (share_mint, _) =
        Pubkey::find_program_address(&[yield_vault::SHARE_MINT_SEED, vault.as_ref()], &program_id);
    let (vault_token, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_TOKEN_SEED, vault.as_ref()], &program_id);

    let ix = Instruction::new_with_bytes(
        program_id,
        &yield_vault::instruction::Withdraw {
            shares: 1_000,
            min_out: 0,
        }
        .data(),
        yield_vault::accounts::Withdraw {
            user: payer.pubkey(),
            vault,
            underlying_mint: fake_mint,
            share_mint,
            vault_token,
            fee_recipient_ata: Keypair::new().pubkey(),
            user_underlying: Keypair::new().pubkey(),
            user_shares: Keypair::new().pubkey(),
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    assert!(send_ix(&mut svm, &payer, ix).is_err());
}

#[test]
fn initialize_rejects_invalid_apy() {
    let program_id = yield_vault::id();
    let authority = Keypair::new();
    let mint = Keypair::new();
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_so_bytes()).unwrap();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    let mint_data = pack_mint(&authority.pubkey(), 6, 0);
    svm.set_account(
        mint.pubkey(),
        Account {
            lamports: rent_exempt(&svm, Mint::LEN),
            data: mint_data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let (vault, _) =
        Pubkey::find_program_address(&[yield_vault::VAULT_SEED, mint.pubkey().as_ref()], &program_id);
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
            authority: authority.pubkey(),
            underlying_mint: mint.pubkey(),
            vault,
            share_mint,
            vault_token,
            token_program: token_program_id(),
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
    );
    assert!(send_ix(&mut svm, &authority, ix).is_err());
}

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    q3_26_vault::{self, STATE, VAULT_SEED},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const DEPOSIT_LAMPORTS: u64 = 500_000_000;
const WITHDRAW_LAMPORTS: u64 = 100_000_000;

struct VaultTest {
    svm: LiteSVM,
    user: Keypair,
    vault_state: Pubkey,
    vault: Pubkey,
}

fn setup() -> VaultTest {
    let program_id = q3_26_vault::id();
    let user = Keypair::new();
    let (vault_state, _) =
        Pubkey::find_program_address(&[STATE, user.pubkey().as_ref()], &program_id);
    let (vault, _) =
        Pubkey::find_program_address(&[VAULT_SEED, user.pubkey().as_ref()], &program_id);

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/q3_26_vault.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&user.pubkey(), 2_000_000_000).unwrap();

    VaultTest {
        svm,
        user,
        vault_state,
        vault,
    }
}

fn send(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) -> bool {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    match svm.send_transaction(tx) {
        Ok(_) => true,
        Err(err) => {
            eprintln!("tx failed: {err:?}\nlogs: {:#?}", err.meta.logs);
            false
        }
    }
}

fn initialize_ix(user: &Keypair, vault_state: Pubkey, vault: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        q3_26_vault::id(),
        &q3_26_vault::instruction::Initialize {}.data(),
        q3_26_vault::accounts::Initialize {
            user: user.pubkey(),
            vault_state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn deposit_ix(user: &Keypair, vault_state: Pubkey, vault: Pubkey, amount: u64) -> Instruction {
    Instruction::new_with_bytes(
        q3_26_vault::id(),
        &q3_26_vault::instruction::Deposit { amount }.data(),
        q3_26_vault::accounts::Deposit {
            user: user.pubkey(),
            vault_state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn withdraw_ix(user: &Keypair, vault_state: Pubkey, vault: Pubkey, amount: u64) -> Instruction {
    Instruction::new_with_bytes(
        q3_26_vault::id(),
        &q3_26_vault::instruction::Withdraw { amount }.data(),
        q3_26_vault::accounts::Withdraw {
            user: user.pubkey(),
            vault_state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn close_ix(user: &Keypair, vault_state: Pubkey, vault: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        q3_26_vault::id(),
        &q3_26_vault::instruction::Close {}.data(),
        q3_26_vault::accounts::Close {
            user: user.pubkey(),
            vault_state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn initialize(test: &mut VaultTest) {
    assert!(
        send(
            &mut test.svm,
            &test.user,
            initialize_ix(&test.user, test.vault_state, test.vault),
        ),
        "initialize should succeed"
    );
}

#[test]
fn initialize_creates_state_and_rent_funded_vault() {
    let mut test = setup();
    initialize(&mut test);

    assert!(
        test.svm.get_account(&test.vault_state).is_some(),
        "vault state should exist after initialize"
    );
    assert_eq!(
        test.svm.get_balance(&test.vault).unwrap(),
        test.svm.minimum_balance_for_rent_exemption(0),
        "vault should be funded to its rent-exempt minimum"
    );

    assert!(
        !send(
            &mut test.svm,
            &test.user,
            initialize_ix(&test.user, test.vault_state, test.vault),
        ),
        "initializing the same state twice should fail"
    );
}

#[test]
fn deposit_adds_lamports_and_rejects_zero() {
    let mut test = setup();
    initialize(&mut test);
    let starting_balance = test.svm.get_balance(&test.vault).unwrap();

    assert!(
        send(
            &mut test.svm,
            &test.user,
            deposit_ix(&test.user, test.vault_state, test.vault, DEPOSIT_LAMPORTS,),
        ),
        "deposit should succeed"
    );
    assert_eq!(
        test.svm.get_balance(&test.vault).unwrap(),
        starting_balance + DEPOSIT_LAMPORTS
    );

    assert!(
        !send(
            &mut test.svm,
            &test.user,
            deposit_ix(&test.user, test.vault_state, test.vault, 0),
        ),
        "zero-value deposit should fail"
    );
    assert_eq!(
        test.svm.get_balance(&test.vault).unwrap(),
        starting_balance + DEPOSIT_LAMPORTS,
        "a rejected deposit must not change the vault"
    );
}

#[test]
fn withdraw_transfers_funds_and_rejects_invalid_amounts() {
    let mut test = setup();
    initialize(&mut test);
    assert!(
        send(
            &mut test.svm,
            &test.user,
            deposit_ix(&test.user, test.vault_state, test.vault, DEPOSIT_LAMPORTS,),
        ),
        "deposit setup should succeed"
    );
    let rent_minimum = test.svm.minimum_balance_for_rent_exemption(0);

    assert!(
        send(
            &mut test.svm,
            &test.user,
            withdraw_ix(&test.user, test.vault_state, test.vault, WITHDRAW_LAMPORTS,),
        ),
        "withdraw should succeed"
    );
    assert_eq!(
        test.svm.get_balance(&test.vault).unwrap(),
        rent_minimum + DEPOSIT_LAMPORTS - WITHDRAW_LAMPORTS
    );

    for amount in [0, DEPOSIT_LAMPORTS] {
        let balance_before = test.svm.get_balance(&test.vault).unwrap();
        assert!(
            !send(
                &mut test.svm,
                &test.user,
                withdraw_ix(&test.user, test.vault_state, test.vault, amount),
            ),
            "zero or rent-consuming withdrawal should fail"
        );
        assert_eq!(
            test.svm.get_balance(&test.vault).unwrap(),
            balance_before,
            "a rejected withdrawal must not change the vault"
        );
    }
}

#[test]
fn close_returns_vault_balance_and_closes_state() {
    let mut test = setup();
    initialize(&mut test);
    assert!(
        send(
            &mut test.svm,
            &test.user,
            deposit_ix(&test.user, test.vault_state, test.vault, DEPOSIT_LAMPORTS,),
        ),
        "deposit setup should succeed"
    );
    assert!(
        send(
            &mut test.svm,
            &test.user,
            withdraw_ix(&test.user, test.vault_state, test.vault, WITHDRAW_LAMPORTS,),
        ),
        "withdraw setup should succeed"
    );

    let user_balance_before_close = test.svm.get_balance(&test.user.pubkey()).unwrap();
    let vault_balance_before_close = test.svm.get_balance(&test.vault).unwrap();
    assert!(
        send(
            &mut test.svm,
            &test.user,
            close_ix(&test.user, test.vault_state, test.vault),
        ),
        "close should succeed"
    );

    assert!(
        test.svm.get_account(&test.vault_state).is_none(),
        "close should reclaim the state account"
    );
    assert_eq!(
        test.svm.get_balance(&test.vault).unwrap_or(0),
        0,
        "close should drain the vault"
    );
    assert!(
        test.svm.get_balance(&test.user.pubkey()).unwrap() > user_balance_before_close,
        "close should return the vault balance and state rent to the user"
    );
    assert!(
        !send(
            &mut test.svm,
            &test.user,
            close_ix(&test.user, test.vault_state, test.vault),
        ),
        "closing an already closed state should fail"
    );
    assert_eq!(
        test.svm.get_balance(&test.vault).unwrap_or(0),
        0,
        "a rejected second close must not recreate or fund the vault"
    );
    assert!(vault_balance_before_close > 0);
}

#[test]
fn another_signer_cannot_deposit_or_close_the_vault() {
    let mut test = setup();
    initialize(&mut test);

    let attacker = Keypair::new();
    test.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();
    let vault_balance_before = test.svm.get_balance(&test.vault).unwrap();

    assert!(
        !send(
            &mut test.svm,
            &attacker,
            deposit_ix(&attacker, test.vault_state, test.vault, DEPOSIT_LAMPORTS),
        ),
        "deposit must reject another user's state and vault PDAs"
    );
    assert!(
        !send(
            &mut test.svm,
            &attacker,
            close_ix(&attacker, test.vault_state, test.vault),
        ),
        "close must reject another user's state and vault PDAs"
    );
    assert_eq!(
        test.svm.get_balance(&test.vault).unwrap(),
        vault_balance_before,
        "rejected instructions must not change the owner's vault"
    );
    assert!(test.svm.get_account(&test.vault_state).is_some());
}

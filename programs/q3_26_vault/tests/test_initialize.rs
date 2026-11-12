use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    q3_26_vault::{STATE, VAULT_SEED},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const DEPOSIT_LAMPORTS: u64 = 500_000_000;
const WITHDRAW_LAMPORTS: u64 = 100_000_000;
const EXCESSIVE_WITHDRAW_LAMPORTS: u64 = 500_000_000;
const ZERO_LAMPORTS: u64 = 0;

fn send(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx).unwrap_or_else(|err| {
        panic!("tx failed: {err:?}\nlogs: {logs:#?}", logs = err.meta.logs);
    });
}

#[test]
fn test() {
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

    send(
        &mut svm,
        &user,
        Instruction::new_with_bytes(
            program_id,
            &q3_26_vault::instruction::Initialize {}.data(),
            q3_26_vault::accounts::Initialize {
                user: user.pubkey(),
                vault_state,
                vault,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        ),
    );

    let rent_exempt = svm.minimum_balance_for_rent_exemption(0);
    assert!(
        svm.get_account(&vault_state).is_some(),
        "vault_state should exist after initialize"
    );
    assert_eq!(
        svm.get_balance(&vault).unwrap(),
        rent_exempt,
        "vault should be rent-exempt after initialize"
    );

    send(
        &mut svm,
        &user,
        Instruction::new_with_bytes(
            program_id,
            &q3_26_vault::instruction::Deposit {
                amount: DEPOSIT_LAMPORTS,
            }
            .data(),
            q3_26_vault::accounts::Deposit {
                user: user.pubkey(),
                vault_state,
                vault,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        ),
    );

    assert_eq!(
        svm.get_balance(&vault).unwrap(),
        rent_exempt + DEPOSIT_LAMPORTS,
        "vault should increase by the deposit"
    );

    send(
        &mut svm,
        &user,
        Instruction::new_with_bytes(
            program_id,
            &q3_26_vault::instruction::Withdraw {
                amount: WITHDRAW_LAMPORTS,
            }
            .data(),
            q3_26_vault::accounts::Withdraw {
                user: user.pubkey(),
                vault_state,
                vault,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        ),
    );

    assert_eq!(
        svm.get_balance(&vault).unwrap(),
        rent_exempt + DEPOSIT_LAMPORTS - WITHDRAW_LAMPORTS,
        "vault should decrease by the withdraw"
    );
    let balance_before_excessive_withdraw = svm.get_balance(&vault).unwrap();

    let excessive_withdraw_ix = Instruction::new_with_bytes(
        program_id,
        &q3_26_vault::instruction::Withdraw {
            amount: EXCESSIVE_WITHDRAW_LAMPORTS,
        }
        .data(),
        q3_26_vault::accounts::Withdraw {
            user: user.pubkey(),
            vault_state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg =
        Message::new_with_blockhash(&[excessive_withdraw_ix], Some(&user.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&user]).unwrap();

    assert!(
        svm.send_transaction(tx).is_err(),
        "excessive withdrawal should fail"
    );

    assert_eq!(
        svm.get_balance(&vault).unwrap(),
        balance_before_excessive_withdraw,
        "vault balance should not change after failed withdrawal"
    );

    // INVALID WITHDRAWAL: ZERO AMOUNT
    // ADDED: Zero-value withdrawals should be rejected by InvalidAmount.
    let zero_withdraw_ix = Instruction::new_with_bytes(
        program_id,
        &q3_26_vault::instruction::Withdraw {
            amount: ZERO_LAMPORTS,
        }
        .data(),
        q3_26_vault::accounts::Withdraw {
            user: user.pubkey(),
            vault_state,
            vault,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[zero_withdraw_ix], Some(&user.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&user]).unwrap();

    assert!(
        svm.send_transaction(tx).is_err(),
        "zero withdrawal should fail"
    );

    //close the vault and return the rent-exempt balance to the user
    send(
        &mut svm,
        &user,
        Instruction::new_with_bytes(
            program_id,
            &q3_26_vault::instruction::Close {}.data(),
            q3_26_vault::accounts::Close {
                user: user.pubkey(),
                vault_state,
                vault,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        ),
    );

    assert!(
        svm.get_account(&vault_state).is_none(),
        "vault_state should be closed"
    );
    assert_eq!(
        svm.get_balance(&vault).unwrap_or(0),
        0,
        "vault should be empty"
    );
}

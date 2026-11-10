use {
    anchor_lang::{
        prelude::rent,
        prelude::Pubkey,
        solana_program::{instruction::Instruction, program_pack::Pack, system_instruction},
        InstructionData, ToAccountMetas,
    },
    anchor_spl::token::spl_token,
    litesvm::LiteSVM,
    q3_26_vault::{self, RECEIPT_VAULT_STATE, SHARE_MINT_SEED, VAULT_SEED},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const INITIAL_RECEIPTS: u64 = 1_000;
const DEPOSIT_RECEIPTS: u64 = 100;

struct VaultTest {
    svm: LiteSVM,
    user: Keypair,
    second_user: Keypair,
    receipt_mint: Keypair,
    user_receipt_account: Keypair,
    second_receipt_account: Keypair,
    vault_state: Pubkey,
    receipt_vault: Pubkey,
    share_mint: Pubkey,
    user_share_account: Option<Keypair>,
    second_share_account: Option<Keypair>,
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instruction: Instruction,
    other_signers: &[&Keypair],
) -> bool {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&dyn Signer> = vec![payer];
    signers.extend(other_signers.iter().map(|signer| *signer as &dyn Signer));
    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers).unwrap();

    match svm.send_transaction(transaction) {
        Ok(_) => true,
        Err(err) => {
            eprintln!("tx failed: {err:?}\nlogs: {:#?}", err.meta.logs);
            false
        }
    }
}

fn create_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Keypair,
    freeze_authority: Option<Pubkey>,
) {
    let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN);
    let create = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        spl_token::state::Mint::LEN as u64,
        &spl_token::ID,
    );
    let initialize = spl_token::instruction::initialize_mint2(
        &spl_token::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        freeze_authority.as_ref(),
        0,
    )
    .unwrap();
    assert!(send(svm, payer, create, &[mint]));
    assert!(send(svm, payer, initialize, &[]));
}

fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    account: &Keypair,
    mint: Pubkey,
    owner: Pubkey,
) {
    let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Account::LEN);
    let create = system_instruction::create_account(
        &payer.pubkey(),
        &account.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::ID,
    );
    let initialize = spl_token::instruction::initialize_account3(
        &spl_token::ID,
        &account.pubkey(),
        &mint,
        &owner,
    )
    .unwrap();
    assert!(send(svm, payer, create, &[account]));
    assert!(send(svm, payer, initialize, &[]));
}

fn mint_to(svm: &mut LiteSVM, authority: &Keypair, mint: Pubkey, account: Pubkey, amount: u64) {
    let instruction = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint,
        &account,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    assert!(send(svm, authority, instruction, &[]));
}

fn token_amount(svm: &LiteSVM, address: Pubkey) -> u64 {
    let account = svm.get_account(&address).expect("token account exists");
    spl_token::state::Account::unpack(&account.data)
        .unwrap()
        .amount
}

fn mint_supply(svm: &LiteSVM, address: Pubkey) -> u64 {
    let account = svm.get_account(&address).expect("mint account exists");
    spl_token::state::Mint::unpack(&account.data)
        .unwrap()
        .supply
}

fn setup(freeze_authority: Option<Pubkey>) -> VaultTest {
    let program_id = q3_26_vault::id();
    let user = Keypair::new();
    let second_user = Keypair::new();
    let receipt_mint = Keypair::new();
    let user_receipt_account = Keypair::new();
    let second_receipt_account = Keypair::new();
    let (vault_state, _) = Pubkey::find_program_address(
        &[RECEIPT_VAULT_STATE, receipt_mint.pubkey().as_ref()],
        &program_id,
    );
    let (receipt_vault, _) =
        Pubkey::find_program_address(&[VAULT_SEED, receipt_mint.pubkey().as_ref()], &program_id);
    let (share_mint, _) = Pubkey::find_program_address(
        &[SHARE_MINT_SEED, receipt_mint.pubkey().as_ref()],
        &program_id,
    );

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/q3_26_vault.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&user.pubkey(), 5_000_000_000).unwrap();
    svm.airdrop(&second_user.pubkey(), 5_000_000_000).unwrap();
    create_mint(&mut svm, &user, &receipt_mint, freeze_authority);
    create_token_account(
        &mut svm,
        &user,
        &user_receipt_account,
        receipt_mint.pubkey(),
        user.pubkey(),
    );
    create_token_account(
        &mut svm,
        &user,
        &second_receipt_account,
        receipt_mint.pubkey(),
        second_user.pubkey(),
    );
    mint_to(
        &mut svm,
        &user,
        receipt_mint.pubkey(),
        user_receipt_account.pubkey(),
        INITIAL_RECEIPTS,
    );
    mint_to(
        &mut svm,
        &user,
        receipt_mint.pubkey(),
        second_receipt_account.pubkey(),
        INITIAL_RECEIPTS,
    );

    VaultTest {
        svm,
        user,
        second_user,
        receipt_mint,
        user_receipt_account,
        second_receipt_account,
        vault_state,
        receipt_vault,
        share_mint,
        user_share_account: None,
        second_share_account: None,
    }
}

fn initialize_ix(test: &VaultTest) -> Instruction {
    Instruction::new_with_bytes(
        q3_26_vault::id(),
        &q3_26_vault::instruction::Initialize {}.data(),
        q3_26_vault::accounts::Initialize {
            payer: test.user.pubkey(),
            receipt_mint: test.receipt_mint.pubkey(),
            vault_state: test.vault_state,
            receipt_vault: test.receipt_vault,
            share_mint: test.share_mint,
            token_program: spl_token::ID,
            system_program: anchor_lang::system_program::ID,
            rent: rent::ID,
        }
        .to_account_metas(None),
    )
}

fn deposit_ix(
    vault_state: Pubkey,
    receipt_mint: Pubkey,
    receipt_vault: Pubkey,
    share_mint: Pubkey,
    user: Pubkey,
    user_receipt_account: Pubkey,
    user_share_account: Pubkey,
    amount: u64,
    min_shares_out: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        q3_26_vault::id(),
        &q3_26_vault::instruction::Deposit {
            amount,
            min_shares_out,
        }
        .data(),
        q3_26_vault::accounts::Deposit {
            user,
            vault_state,
            receipt_mint,
            receipt_vault,
            share_mint,
            user_receipt_account,
            user_share_account,
            token_program: spl_token::ID,
        }
        .to_account_metas(None),
    )
}

fn redeem_ix(
    vault_state: Pubkey,
    receipt_mint: Pubkey,
    receipt_vault: Pubkey,
    share_mint: Pubkey,
    user: Pubkey,
    user_share_account: Pubkey,
    user_receipt_account: Pubkey,
    shares: u64,
    min_receipts_out: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        q3_26_vault::id(),
        &q3_26_vault::instruction::Redeem {
            shares,
            min_receipts_out,
        }
        .data(),
        q3_26_vault::accounts::Redeem {
            user,
            vault_state,
            receipt_mint,
            receipt_vault,
            share_mint,
            user_share_account,
            user_receipt_account,
            token_program: spl_token::ID,
        }
        .to_account_metas(None),
    )
}

fn initialize(test: &mut VaultTest) -> bool {
    let ix = initialize_ix(test);
    send(&mut test.svm, &test.user, ix, &[])
}

fn create_share_accounts(test: &mut VaultTest) {
    let user_account = Keypair::new();
    let second_account = Keypair::new();
    create_token_account(
        &mut test.svm,
        &test.user,
        &user_account,
        test.share_mint,
        test.user.pubkey(),
    );
    create_token_account(
        &mut test.svm,
        &test.user,
        &second_account,
        test.share_mint,
        test.second_user.pubkey(),
    );
    test.user_share_account = Some(user_account);
    test.second_share_account = Some(second_account);
}

#[test]
fn initialize_creates_shared_vault_and_share_mint_once() {
    let mut test = setup(None);
    assert!(initialize(&mut test), "initialization should succeed");
    assert!(test.svm.get_account(&test.vault_state).is_some());
    assert_eq!(token_amount(&test.svm, test.receipt_vault), 0);
    assert_eq!(mint_supply(&test.svm, test.share_mint), 0);
    assert!(
        !initialize(&mut test),
        "a receipt mint must have at most one vault"
    );
}

#[test]
fn initialize_rejects_receipt_mints_with_freeze_authority() {
    let mut test = setup(Some(Pubkey::new_unique()));
    assert!(
        !initialize(&mut test),
        "freeze authority could prevent users from transferring receipts on exit"
    );
    assert!(test.svm.get_account(&test.vault_state).is_none());
}

#[test]
fn deposit_mints_shares_and_rejects_zero_or_excessive_minimum() {
    let mut test = setup(None);
    assert!(initialize(&mut test));
    create_share_accounts(&mut test);
    let share_account = test.user_share_account.as_ref().unwrap().pubkey();
    let source_before = token_amount(&test.svm, test.user_receipt_account.pubkey());

    assert!(
        !send(
            &mut test.svm,
            &test.user,
            deposit_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.user.pubkey(),
                test.user_receipt_account.pubkey(),
                share_account,
                DEPOSIT_RECEIPTS,
                DEPOSIT_RECEIPTS + 1,
            ),
            &[],
        ),
        "minimum shares-out must be enforced"
    );
    assert_eq!(token_amount(&test.svm, test.receipt_vault), 0);
    assert_eq!(
        token_amount(&test.svm, test.user_receipt_account.pubkey()),
        source_before,
        "a rejected deposit must roll back its receipt transfer"
    );

    assert!(
        send(
            &mut test.svm,
            &test.user,
            deposit_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.user.pubkey(),
                test.user_receipt_account.pubkey(),
                share_account,
                DEPOSIT_RECEIPTS,
                DEPOSIT_RECEIPTS,
            ),
            &[],
        ),
        "first deposit should mint shares one-for-one"
    );
    assert_eq!(
        token_amount(&test.svm, test.receipt_vault),
        DEPOSIT_RECEIPTS
    );
    assert_eq!(token_amount(&test.svm, share_account), DEPOSIT_RECEIPTS);
    assert_eq!(mint_supply(&test.svm, test.share_mint), DEPOSIT_RECEIPTS);

    assert!(
        !send(
            &mut test.svm,
            &test.user,
            deposit_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.user.pubkey(),
                test.user_receipt_account.pubkey(),
                share_account,
                0,
                0,
            ),
            &[],
        ),
        "zero deposits should fail"
    );
}

#[test]
fn users_redeem_pro_rata_receipt_tokens_without_market_liquidity() {
    let mut test = setup(None);
    assert!(initialize(&mut test));
    create_share_accounts(&mut test);
    let user_shares = test.user_share_account.as_ref().unwrap().pubkey();
    let second_shares = test.second_share_account.as_ref().unwrap().pubkey();

    for (user, receipts, shares, payer) in [
        (
            test.user.pubkey(),
            test.user_receipt_account.pubkey(),
            user_shares,
            &test.user,
        ),
        (
            test.second_user.pubkey(),
            test.second_receipt_account.pubkey(),
            second_shares,
            &test.second_user,
        ),
    ] {
        assert!(
            send(
                &mut test.svm,
                payer,
                deposit_ix(
                    test.vault_state,
                    test.receipt_mint.pubkey(),
                    test.receipt_vault,
                    test.share_mint,
                    user,
                    receipts,
                    shares,
                    DEPOSIT_RECEIPTS,
                    DEPOSIT_RECEIPTS,
                ),
                &[],
            ),
            "both users should be able to deposit into the shared vault"
        );
    }

    assert_eq!(token_amount(&test.svm, test.receipt_vault), 200);
    assert_eq!(mint_supply(&test.svm, test.share_mint), 200);

    let donation = spl_token::instruction::transfer(
        &spl_token::ID,
        &test.user_receipt_account.pubkey(),
        &test.receipt_vault,
        &test.user.pubkey(),
        &[],
        1,
    )
    .unwrap();
    assert!(send(&mut test.svm, &test.user, donation, &[]));
    assert_eq!(token_amount(&test.svm, test.receipt_vault), 201);

    let second_receipts_before = token_amount(&test.svm, test.second_receipt_account.pubkey());
    assert!(
        !send(
            &mut test.svm,
            &test.second_user,
            redeem_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.second_user.pubkey(),
                second_shares,
                test.second_receipt_account.pubkey(),
                DEPOSIT_RECEIPTS,
                DEPOSIT_RECEIPTS + 1,
            ),
            &[],
        ),
        "minimum receipts-out must be enforced atomically"
    );
    assert_eq!(token_amount(&test.svm, test.receipt_vault), 201);
    assert_eq!(token_amount(&test.svm, second_shares), 100);
    assert_eq!(
        token_amount(&test.svm, test.second_receipt_account.pubkey()),
        second_receipts_before
    );

    assert!(
        send(
            &mut test.svm,
            &test.second_user,
            redeem_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.second_user.pubkey(),
                second_shares,
                test.second_receipt_account.pubkey(),
                DEPOSIT_RECEIPTS,
                DEPOSIT_RECEIPTS,
            ),
            &[],
        ),
        "redemption must pay receipt tokens even if the market has no SOL liquidity"
    );
    assert_eq!(token_amount(&test.svm, test.receipt_vault), 101);
    assert_eq!(
        token_amount(&test.svm, test.second_receipt_account.pubkey()),
        second_receipts_before + 100
    );
    assert_eq!(mint_supply(&test.svm, test.share_mint), 100);

    assert!(
        send(
            &mut test.svm,
            &test.user,
            redeem_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.user.pubkey(),
                user_shares,
                test.user_receipt_account.pubkey(),
                DEPOSIT_RECEIPTS,
                101,
            ),
            &[],
        ),
        "last share-holder receives all remaining receipt-token dust"
    );
    assert_eq!(token_amount(&test.svm, test.receipt_vault), 0);
    assert_eq!(mint_supply(&test.svm, test.share_mint), 0);
    assert_eq!(
        token_amount(&test.svm, test.user_receipt_account.pubkey()),
        INITIAL_RECEIPTS
    );
}

#[test]
fn users_cannot_redeem_another_users_shares_or_zero_shares() {
    let mut test = setup(None);
    assert!(initialize(&mut test));
    create_share_accounts(&mut test);
    let owner_shares = test.user_share_account.as_ref().unwrap().pubkey();
    let attacker_shares = test.second_share_account.as_ref().unwrap().pubkey();
    assert!(
        send(
            &mut test.svm,
            &test.user,
            deposit_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.user.pubkey(),
                test.user_receipt_account.pubkey(),
                owner_shares,
                DEPOSIT_RECEIPTS,
                DEPOSIT_RECEIPTS,
            ),
            &[],
        ),
        "deposit setup should succeed"
    );

    assert!(
        !send(
            &mut test.svm,
            &test.second_user,
            redeem_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.second_user.pubkey(),
                owner_shares,
                test.second_receipt_account.pubkey(),
                DEPOSIT_RECEIPTS,
                0,
            ),
            &[],
        ),
        "a user must not redeem shares held by another user"
    );
    assert!(
        !send(
            &mut test.svm,
            &test.user,
            redeem_ix(
                test.vault_state,
                test.receipt_mint.pubkey(),
                test.receipt_vault,
                test.share_mint,
                test.user.pubkey(),
                owner_shares,
                test.user_receipt_account.pubkey(),
                0,
                0,
            ),
            &[],
        ),
        "zero-share redemption should fail"
    );
    assert_eq!(
        token_amount(&test.svm, test.receipt_vault),
        DEPOSIT_RECEIPTS
    );
    assert_eq!(token_amount(&test.svm, attacker_shares), 0);
}

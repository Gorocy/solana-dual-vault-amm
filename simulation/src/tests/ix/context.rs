use solana_program::example_mocks::solana_sdk::system_instruction;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use crate::utils::ix::context::SimContext;

#[test]
fn test_initialization() {
    let ctx = SimContext::new();

    let payer_pubkey = ctx.payer.pubkey();
    assert_ne!(
        payer_pubkey,
        Pubkey::default(),
        "Payer should have a non-zero public key"
    );

    // Check if the list of programs is empty at the beginning
    assert!(
        ctx.programs.is_empty(),
        "The list of programs should be empty after initialization"
    );

    // Check if the payer has zero balance at the beginning
    let account = ctx.svm.get_account(&payer_pubkey);
    assert!(
        account.is_none(),
        "Payer should not have an account before airdrop"
    );
}

#[test]
fn test_airdrop_payer() {
    let mut ctx = SimContext::new();
    let amount = 5 * LAMPORTS_PER_SOL;

    let result = ctx.airdrop_payer(amount);
    assert!(result.is_ok(), "Airdrop should succeed");

    let payer_pubkey = ctx.payer.pubkey();
    let account = ctx.svm.get_account(&payer_pubkey);

    assert!(account.is_some(), "Account should exist after airdrop");
    assert_eq!(
        account.unwrap().lamports,
        amount,
        "Account balance does not match airdrop amount"
    );
}

#[test]
fn test_airdrop_multiple_times() {
    let mut ctx = SimContext::new();
    let first_amount = 2 * LAMPORTS_PER_SOL;
    let second_amount = 3 * LAMPORTS_PER_SOL;

    // First airdrop
    ctx.airdrop_payer(first_amount).unwrap();

    let payer_pubkey = ctx.payer.pubkey();
    let account = ctx.svm.get_account(&payer_pubkey).unwrap();

    assert_eq!(
        account.lamports,
        first_amount,
        "Balance should be equal to first airdrop amount"
    );

    // Second airdrop
    ctx.airdrop_payer(second_amount).unwrap();

    let payer_pubkey = ctx.payer.pubkey();
    let account = ctx.svm.get_account(&payer_pubkey).unwrap();


    assert_eq!(
        account.lamports,
        second_amount,
        "Balance should be equal to second airdrop amount"
    );
}

#[test]
fn test_airdrop_zero_amount() {
    let mut ctx = SimContext::new();
    let result = ctx.airdrop_payer(0);

    // Airdrop zero should succeed
    assert!(
        result.is_ok(),
        "Airdrop zero lamports should succeed"
    );

    let payer_pubkey = ctx.payer.pubkey();
    let account = ctx.svm.get_account(&payer_pubkey);

    // But the account may not be created or have 0 lamports
    if let Some(acc) = account {
        assert_eq!(acc.lamports, 0, "Account should have 0 lamports");
    }
}

#[test]
fn test_send_system_transfer() {
    let mut ctx = SimContext::new();

    let initial_balance = 10 * LAMPORTS_PER_SOL;
    ctx.airdrop_payer(initial_balance).unwrap();

    let recipient = Keypair::new();
    let transfer_amount = 1 * LAMPORTS_PER_SOL;

    let transfer_ix =
        system_instruction::transfer(&ctx.payer.pubkey(), &recipient.pubkey(), transfer_amount);

    let result = ctx.send_tx(&[transfer_ix], None);
    assert!(
        result.is_ok(),
        "System transfer transaction failed: {:?}",
        result.err()
    );

    let recipient_acc = ctx.svm.get_account(&recipient.pubkey());
    assert!(
        recipient_acc.is_some(),
        "Recipient account should be created"
    );
    assert_eq!(
        recipient_acc.unwrap().lamports,
        transfer_amount,
        "Recipient did not receive SOL"
    );

    let payer_acc = ctx.svm.get_account(&ctx.payer.pubkey()).unwrap();
    assert!(
        payer_acc.lamports < initial_balance - transfer_amount,
        "Payer did not pay for gas fee"
    );
}

#[test]
fn test_send_tx_with_custom_signer() {
    let mut ctx = SimContext::new();
    ctx.airdrop_payer(1 * LAMPORTS_PER_SOL).unwrap();

    let authority = Keypair::new();

    ctx.svm
        .airdrop(&authority.pubkey(), 1 * LAMPORTS_PER_SOL)
        .unwrap();

    let recipient = Pubkey::new_unique();

    let ix = system_instruction::transfer(&authority.pubkey(), &recipient, 1000);

    let payer = Keypair::from_base58_string(&ctx.payer.to_base58_string());
    let result = ctx.send_tx(&[ix], Some(&[&payer, &authority]));
    assert!(result.is_ok());
}

#[test]
fn test_send_tx_insufficient_funds() {
    let mut ctx = SimContext::new();

    // We do not airdrop, so payer has 0 lamports
    let recipient = Keypair::new();
    let transfer_amount = 1 * LAMPORTS_PER_SOL;

    let transfer_ix =
        system_instruction::transfer(&ctx.payer.pubkey(), &recipient.pubkey(), transfer_amount);

    let result = ctx.send_tx(&[transfer_ix], None);
    assert!(
        result.is_err(),
        "Transaction should fail due to insufficient funds"
    );
}

#[test]
fn test_send_multiple_instructions() {
    let mut ctx = SimContext::new();
    let initial_balance = 10 * LAMPORTS_PER_SOL;
    ctx.airdrop_payer(initial_balance).unwrap();

    let recipient1 = Keypair::new();
    let recipient2 = Keypair::new();
    let transfer_amount = 1 * LAMPORTS_PER_SOL;

    let instructions = vec![
        system_instruction::transfer(&ctx.payer.pubkey(), &recipient1.pubkey(), transfer_amount),
        system_instruction::transfer(&ctx.payer.pubkey(), &recipient2.pubkey(), transfer_amount),
    ];

    let result = ctx.send_tx(&instructions, None);
    assert!(
        result.is_ok(),
        "Transaction with multiple instructions should succeed"
    );

    // Check if both recipients received funds
    let recipient1_acc = ctx.svm.get_account(&recipient1.pubkey()).unwrap();
    let recipient2_acc = ctx.svm.get_account(&recipient2.pubkey()).unwrap();

    assert_eq!(recipient1_acc.lamports, transfer_amount);
    assert_eq!(recipient2_acc.lamports, transfer_amount);
}

#[test]
fn test_send_empty_instructions() {
    let mut ctx = SimContext::new();
    ctx.airdrop_payer(1 * LAMPORTS_PER_SOL).unwrap();

    let result = ctx.send_tx(&[], None);
    // Transaction with no instructions may succeed (but does nothing)
    assert!(
        result.is_ok(),
        "Transaction with no instructions should succeed"
    );
}

#[test]
fn test_deploy_program() {
    let mut ctx = SimContext::new();
    let program_id = Pubkey::new_unique();

    let program_name = "cpmm_rebalansing";

    ctx.deploy_program(program_name, program_id);

    assert!(ctx.programs.contains(&program_id));

    let acc = ctx.svm.get_account(&program_id).unwrap();
    assert!(acc.executable, "Deployed account should be executable");
}

#[test]
fn test_deploy_multiple_programs() {
    let mut ctx = SimContext::new();
    let program_id1 = Pubkey::new_unique();
    let program_id2 = Pubkey::new_unique();

    let program_name = "cpmm_rebalansing";

    ctx.deploy_program(program_name, program_id1);
    ctx.deploy_program(program_name, program_id2);

    let programs = ctx.programs;
    assert_eq!(programs.len(), 2, "There should be 2 installed programs");
    assert!(programs.contains(&program_id1));
    assert!(programs.contains(&program_id2));

    // Check if both programs are executable
    let acc1 = ctx.svm.get_account(&program_id1).unwrap();
    let acc2 = ctx.svm.get_account(&program_id2).unwrap();
    assert!(acc1.executable);
    assert!(acc2.executable);
}

#[test]
#[should_panic(expected = "Not Found file")]
fn test_deploy_nonexistent_program() {
    let mut ctx = SimContext::new();
    let program_id = Pubkey::new_unique();

    // Attempt to deploy a nonexistent program
    ctx.deploy_program("nonexistent_program", program_id);
}

#[test]
fn test_compute_units_tracking() {
    let mut ctx = SimContext::new();
    let initial_balance = 10 * LAMPORTS_PER_SOL;
    ctx.airdrop_payer(initial_balance).unwrap();

    let recipient = Keypair::new();
    let transfer_amount = 1000; // Small amount for a simple transaction

    let transfer_ix =
        system_instruction::transfer(&ctx.payer.pubkey(), &recipient.pubkey(), transfer_amount);

    let result = ctx.send_tx(&[transfer_ix], None);
    assert!(result.is_ok());

    let compute_units = result.unwrap();
    assert!(compute_units > 0, "Transaction should consume compute units");
    println!("Consumed compute units: {}", compute_units);
}

#[test]
fn test_invalid_instruction() {
    let mut ctx = SimContext::new();
    ctx.airdrop_payer(1 * LAMPORTS_PER_SOL).unwrap();

    // Create an invalid instruction (nonexistent program)
    let fake_program_id = Pubkey::new_unique();
    let invalid_ix = Instruction {
        program_id: fake_program_id,
        accounts: vec![AccountMeta::new(ctx.payer.pubkey(), true)],
        data: vec![0, 1, 2, 3], // Some data
    };

    let result = ctx.send_tx(&[invalid_ix], None);
    assert!(
        result.is_err(),
        "Transaction with an invalid instruction should fail"
    );
}

#[test]
fn test_context_isolation() {
    // Test checking if different contexts are independent from each other
    let mut ctx1 = SimContext::new();
    let ctx2 = SimContext::new();

    ctx1.airdrop_payer(5 * LAMPORTS_PER_SOL).unwrap();

    // ctx2 should not have access to the account from ctx1
    let payer1_pubkey = ctx1.payer.pubkey();
    let account_in_ctx2 = ctx2.svm.get_account(&payer1_pubkey);

    assert!(
        account_in_ctx2.is_none(),
        "Contexts should be independent from each other"
    );
}

#[test]
fn test_payer_reference_consistency() {
    let ctx = SimContext::new();

    // Check if payer is consistent between calls
    let payer1 = ctx.payer.pubkey();
    let payer2 = ctx.payer.pubkey();

    assert_eq!(payer1, payer2, "Payer pubkey should be consistent");
}

#[test]
fn test_programs_list_consistency() {
    let mut ctx = SimContext::new();
    let program_id = Pubkey::new_unique();

    // Check the list before adding
    assert_eq!(ctx.programs.len(), 0);

    ctx.deploy_program("cpmm_rebalansing", program_id);

    // Check the list after adding
    assert_eq!(ctx.programs.len(), 1);
    assert_eq!(ctx.programs[0], program_id);
}

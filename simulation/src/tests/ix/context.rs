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
        "Payer powinien mieć niezerowy klucz publiczny"
    );

    // Sprawdź czy lista programów jest pusta na początku
    assert!(
        ctx.programs.is_empty(),
        "Lista programów powinna być pusta po inicjalizacji"
    );

    // Sprawdź czy payer ma zerowy balans na początku
    let account = ctx.svm.get_account(&payer_pubkey);
    assert!(
        account.is_none(),
        "Payer nie powinien mieć konta przed airdropem"
    );
}

#[test]
fn test_airdrop_payer() {
    let mut ctx = SimContext::new();
    let amount = 5 * LAMPORTS_PER_SOL;

    let result = ctx.airdrop_payer(amount);
    assert!(result.is_ok(), "Airdrop zakończył się błędem");

    let payer_pubkey = ctx.payer.pubkey();
    let account = ctx.svm.get_account(&payer_pubkey);

    assert!(account.is_some(), "Konto powinno istnieć po airdropie");
    assert_eq!(
        account.unwrap().lamports,
        amount,
        "Balans konta nie zgadza się z kwotą airdropa"
    );
}

#[test]
fn test_airdrop_multiple_times() {
    let mut ctx = SimContext::new();
    let first_amount = 2 * LAMPORTS_PER_SOL;
    let second_amount = 3 * LAMPORTS_PER_SOL;

    // Pierwszy airdrop
    ctx.airdrop_payer(first_amount).unwrap();

    // Drugi airdrop
    ctx.airdrop_payer(second_amount).unwrap();

    let payer_pubkey = ctx.payer.pubkey();
    let account = ctx.svm.get_account(&payer_pubkey).unwrap();

    assert_eq!(
        account.lamports,
        first_amount + second_amount,
        "Balans powinien być sumą wszystkich airdropów"
    );
}

#[test]
fn test_airdrop_zero_amount() {
    let mut ctx = SimContext::new();
    let result = ctx.airdrop_payer(0);

    // Airdrop zero powinien się powieść
    assert!(
        result.is_ok(),
        "Airdrop zero lamportów powinien się powieść"
    );

    let payer_pubkey = ctx.payer.pubkey();
    let account = ctx.svm.get_account(&payer_pubkey);

    // Ale konto może nie zostać utworzone lub mieć 0 lamportów
    if let Some(acc) = account {
        assert_eq!(acc.lamports, 0, "Konto powinno mieć 0 lamportów");
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
        "Transakcja transferu nie powiodła się: {:?}",
        result.err()
    );

    let recipient_acc = ctx.svm.get_account(&recipient.pubkey());
    assert!(
        recipient_acc.is_some(),
        "Konto odbiorcy powinno zostać utworzone"
    );
    assert_eq!(
        recipient_acc.unwrap().lamports,
        transfer_amount,
        "Odbiorca nie otrzymał SOL"
    );

    let payer_acc = ctx.svm.get_account(&ctx.payer.pubkey()).unwrap();
    assert!(
        payer_acc.lamports < initial_balance - transfer_amount,
        "Payer nie zapłacił za gas fee"
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

    // Nie robimy airdropa, więc payer ma 0 lamportów
    let recipient = Keypair::new();
    let transfer_amount = 1 * LAMPORTS_PER_SOL;

    let transfer_ix =
        system_instruction::transfer(&ctx.payer.pubkey(), &recipient.pubkey(), transfer_amount);

    let result = ctx.send_tx(&[transfer_ix], None);
    assert!(
        result.is_err(),
        "Transakcja powinna się nie powieść z powodu braku funduszy"
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
        "Transakcja z wieloma instrukcjami powinna się powieść"
    );

    // Sprawdź czy oba odbiorcy otrzymali środki
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
    // Transakcja bez instrukcji może się powieść (ale nic nie robi)
    assert!(
        result.is_ok(),
        "Transakcja bez instrukcji powinna się powieść"
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
    assert!(acc.executable, "Wgrane konto powinno być wykonywalne");
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
    assert_eq!(programs.len(), 2, "Powinny być zainstalowane 2 programy");
    assert!(programs.contains(&program_id1));
    assert!(programs.contains(&program_id2));

    // Sprawdź czy oba programy są executable
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

    // Próba zainstalowania nieistniejącego programu
    ctx.deploy_program("nonexistent_program", program_id);
}

#[test]
fn test_compute_units_tracking() {
    let mut ctx = SimContext::new();
    let initial_balance = 10 * LAMPORTS_PER_SOL;
    ctx.airdrop_payer(initial_balance).unwrap();

    let recipient = Keypair::new();
    let transfer_amount = 1000; // Mała kwota dla prostej transakcji

    let transfer_ix =
        system_instruction::transfer(&ctx.payer.pubkey(), &recipient.pubkey(), transfer_amount);

    let result = ctx.send_tx(&[transfer_ix], None);
    assert!(result.is_ok());

    let compute_units = result.unwrap();
    assert!(compute_units > 0, "Transakcja powinna zużyć compute units");
    println!("Zużyte compute units: {}", compute_units);
}

#[test]
fn test_invalid_instruction() {
    let mut ctx = SimContext::new();
    ctx.airdrop_payer(1 * LAMPORTS_PER_SOL).unwrap();

    // Stwórz nieprawidłową instrukcję (nieistniejący program)
    let fake_program_id = Pubkey::new_unique();
    let invalid_ix = Instruction {
        program_id: fake_program_id,
        accounts: vec![AccountMeta::new(ctx.payer.pubkey(), true)],
        data: vec![0, 1, 2, 3], // Jakieś dane
    };

    let result = ctx.send_tx(&[invalid_ix], None);
    assert!(
        result.is_err(),
        "Transakcja z nieprawidłową instrukcją powinna się nie powieść"
    );
}

#[test]
fn test_context_isolation() {
    // Test sprawdzający czy różne konteksty są od siebie niezależne
    let mut ctx1 = SimContext::new();
    let ctx2 = SimContext::new();

    ctx1.airdrop_payer(5 * LAMPORTS_PER_SOL).unwrap();

    // ctx2 nie powinien mieć dostępu do konta z ctx1
    let payer1_pubkey = ctx1.payer.pubkey();
    let account_in_ctx2 = ctx2.svm.get_account(&payer1_pubkey);

    assert!(
        account_in_ctx2.is_none(),
        "Konteksty powinny być od siebie niezależne"
    );
}

#[test]
fn test_payer_reference_consistency() {
    let ctx = SimContext::new();

    // Sprawdź czy payer jest konsystentny między wywołaniami
    let payer1 = ctx.payer.pubkey();
    let payer2 = ctx.payer.pubkey();

    assert_eq!(payer1, payer2, "Pubkey payera powinien być konsystentny");
}

#[test]
fn test_programs_list_consistency() {
    let mut ctx = SimContext::new();
    let program_id = Pubkey::new_unique();

    // Sprawdź listę przed dodaniem
    assert_eq!(ctx.programs.len(), 0);

    ctx.deploy_program("cpmm_rebalansing", program_id);

    // Sprawdź listę po dodaniu
    assert_eq!(ctx.programs.len(), 1);
    assert_eq!(ctx.programs[0], program_id);
}

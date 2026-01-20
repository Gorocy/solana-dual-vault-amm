use derivative::Derivative;
use litesvm::LiteSVM;
use solana_sdk::{
    clock::Clock,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::path::PathBuf;
use tracing::{info, trace};

use crate::error::{ResultSimulation, SimulationError};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SimContext {
    #[derivative(Debug = "ignore")]
    pub svm: LiteSVM,

    pub payer: Keypair,
    pub programs: Vec<Pubkey>,
}

impl Clone for SimContext {
    fn clone(&self) -> Self {
        Self {
            svm: self.svm.clone(),
            payer: self.payer.insecure_clone(),
            programs: self.programs.clone(),
        }
    }
}

impl SimContext {
    pub fn new() -> Self {
        let svm = LiteSVM::new();
        let payer = Keypair::new();
        let programs = Vec::new();
        Self {
            svm,
            payer,
            programs,
        }
    }

    pub fn airdrop(&mut self, pubkey: &Pubkey, amount: u64) -> ResultSimulation<()> {
        self.svm
            .airdrop(pubkey, amount)
            .map_err(SimulationError::FailedTransactionMetadata)?;
        Ok(())
    }

    pub fn airdrop_payer(&mut self, amount: u64) -> ResultSimulation<()> {
        let payer_pubkey = self.payer.pubkey();
        
        if let Some(mut account) = self.svm.get_account(&payer_pubkey) {
            account.lamports = amount;
            self.svm.set_account(payer_pubkey, account)
                .map_err(SimulationError::from)?;
        } else {
            self.svm.airdrop(&payer_pubkey, amount)
                .map_err(SimulationError::FailedTransactionMetadata)?;
        }
        Ok(())
    }

    pub fn deploy_program(&mut self, program_name: &str, program_id: Pubkey) {
        let mut so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        so_path.pop();
        so_path.push("target/deploy");
        so_path.push(format!("{}.so", program_name));
        let program_data =
            std::fs::read(&so_path).expect(&format!("Not Found file: {:?}", so_path));
            
        self.svm
            .add_program(program_id, &program_data)
            .expect(&format!("Error adding program from file: {:?}", so_path));

        info!("Deployed {} at {}", program_name, program_id);
        self.programs.push(program_id);
    }

    pub fn send_tx(
        &mut self,
        instructions: &[Instruction],
        additional_signers: Option<&[&Keypair]>,
    ) -> ResultSimulation<u64> {
        trace!("Will create TX");

        let mut signers: Vec<&Keypair> = vec![&self.payer];

        if let Some(extra) = additional_signers {
            trace!("There are additional signers");
            signers.extend_from_slice(extra);
        }

        let tx = Transaction::new(
            &signers,
            Message::new(instructions, Some(&self.payer.pubkey())),
            self.svm.latest_blockhash(),
        );

        trace!("Will send tx");

        match self.svm.send_transaction(tx) {
            Ok(meta) => Ok(meta.compute_units_consumed),
            Err(e) => Err(SimulationError::FailedTransactionMetadata(e)),
        }
    }

    pub fn next_slot(&mut self) {
        let mut clock = self.svm.get_sysvar::<Clock>();
        clock.slot += 1;

        self.svm.set_sysvar::<Clock>(&clock);
        self.svm.expire_blockhash();
    }
}

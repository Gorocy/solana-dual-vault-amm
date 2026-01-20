use anyhow::Result;
use clap::Parser;
use simulation::utils::{
    args::args_shared::SharedArgs,
    enviroment::new_single_simulation,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    // Parse CLI arguments first to check for --quiet flag
    let args = SharedArgs::parse();
    
    // Setup logging only if not in quiet mode
    // Without subscriber, all tracing macros are no-op
    if !args.quiet {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .with_target(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
    }
    
    info!("Parsed args: {:?}", args);
    
    let mut config = args.into_config_with_strategy(1, None);
    
    // Set single-specific default filenames if not overridden
    if config.output_file == "vault_reserves.csv" {
        config.output_file = "single_vault_reserves.csv".into();
    }
    if config.compute_csv_file == "vault_reserves_compute.csv" {
        config.compute_csv_file = "single_vault_reserves_single_swap_compute.csv".into();
    }

    info!("Starting Vault Trading Simulation");

    // Run simulation
    let simulation = new_single_simulation(config.clone())?;
    config.log_info();

    let results = simulation.run()?;

    // Display results
    results.display();

    Ok(())
}

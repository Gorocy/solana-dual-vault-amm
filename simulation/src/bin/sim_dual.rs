use anyhow::Result;
use clap::Parser;
use simulation::utils::{
    args::args_shared::SharedArgs,
    enviroment::new_dual_simulation,
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

    let mut config = args.into_config_with_strategy(2, Some(2));
    
    // Set dual-specific defaults if not overridden
    if config.output_file == "vault_reserves.csv" {
        config.output_file = "dual_vault_reserves.csv".into();
    }
    if config.compute_csv_file == "vault_reserves_compute.csv" {
        config.compute_csv_file = "dual_vault_reserves_dual_swap_compute.csv".into();
    }

    info!("Starting Dual Vault Trading Simulation");
    info!("Using ONLY dual swap operations between vault pairs");
    config.log_info();

    // Run simulation through common runner for unified output/statistics
    let simulation = new_dual_simulation(config)?;
    let results = simulation.run()?;
    results.display();

    Ok(())
}

use anyhow::Result;
use rayon::prelude::*;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};

const MONTE_CARLO_RUNS: usize = 1_000;
const SEED_BASE: u64 = 42;

#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    pub name: String,
    pub description: String,
    pub is_dual: bool,
    pub enable_arbitrageurs: bool,
}

pub fn get_scenarios() -> Vec<ScenarioConfig> {
    vec![
        ScenarioConfig {
            name: "single_no_arb".to_string(),
            description: "Single Vault + User Noise".to_string(),
            is_dual: false,
            enable_arbitrageurs: false,
        },
        ScenarioConfig {
            name: "single".to_string(),
            description: "Single Vault + Arbitrage".to_string(),
            is_dual: false,
            enable_arbitrageurs: true,
        },
        ScenarioConfig {
            name: "dual_no_arb".to_string(),
            description: "Dual Vault + User Noise".to_string(),
            is_dual: true,
            enable_arbitrageurs: false,
        },
        ScenarioConfig {
            name: "dual".to_string(),
            description: "Dual Vault + Arbitrage".to_string(),
            is_dual: true,
            enable_arbitrageurs: true,
        },
    ]
}

pub fn run_scenario_monte_carlo(scenario_index: usize, user_count: usize) -> Result<()> {
    let scenarios = get_scenarios();
    
    if scenario_index >= scenarios.len() {
        anyhow::bail!("Invalid scenario index: {}", scenario_index);
    }
    
    let scenario = &scenarios[scenario_index];
    
    info!("========================================");
    info!("Monte Carlo Simulation");
    info!("{}", scenario.description);
    info!("Users: {}, Runs: {}", user_count, MONTE_CARLO_RUNS);
    info!("========================================");

    // Create directory structure
    let scenario_dir = format!("{}/{}", user_count, scenario.name);
    fs::create_dir_all(&scenario_dir)?;

    // Progress tracking - report after each completed simulation
    let completed = Arc::new(AtomicU32::new(0));
    let start_time = Instant::now();

    // Parallelize the 1,000 Monte Carlo runs - each thread returns its result independently
    let final_results: Vec<(usize, bool, String)> = (0..MONTE_CARLO_RUNS)
        .into_par_iter()
        .map(|run_idx| {
            let seed = SEED_BASE.wrapping_add(run_idx as u64);
            let result = run_scenario(scenario, user_count, seed);
            
            // Update progress counter and display after each completion
            let count = completed.fetch_add(1, Ordering::Relaxed) + 1;
            let elapsed = start_time.elapsed().as_secs_f64();
            let avg_time_per_run = elapsed / count as f64;
            let remaining_runs = MONTE_CARLO_RUNS - count as usize;
            let eta_seconds = avg_time_per_run * remaining_runs as f64;
            
            info!(
                "[{}/{}] Completed seed {} ({:.1}%) | Avg: {:.1}s/run | ETA: {:.0}s",
                count,
                MONTE_CARLO_RUNS,
                seed,
                (count as f64 / MONTE_CARLO_RUNS as f64) * 100.0,
                avg_time_per_run,
                eta_seconds
            );
            
            match result {
                Ok(_) => (run_idx, true, String::new()),
                Err(e) => {
                    error!("[seed {}]: FAILED - {}", seed, e);
                    (run_idx, false, e.to_string())
                }
            }
        })
        .collect();
    let success_count = final_results.iter().filter(|(_, success, _)| *success).count();
    let failure_count = final_results.len() - success_count;
    
    info!("========================================");
    info!("Simulation Complete");
    info!("========================================");
    info!("Total runs: {}", final_results.len());
    info!("Success: {}", success_count);
    info!("Failed: {}", failure_count);
    info!("========================================");

    if failure_count > 0 {
        anyhow::bail!("{} simulation(s) failed", failure_count);
    }

    Ok(())
}

fn run_scenario(config: &ScenarioConfig, user_count: usize, seed: u64) -> Result<()> {
    let binary = if config.is_dual {
        "target/release/sim_dual"
    } else {
        "target/release/sim_standard"
    };

    let output_file = format!("{}/{}/seed_{}.csv", user_count, config.name, seed);
    let compute_file = format!("{}/{}/seed_{}_compute.csv", user_count, config.name, seed);

    let mut cmd = Command::new(binary);
    cmd.arg("--user-count")
        .arg(user_count.to_string())
        .arg("--output-file")
        .arg(&output_file)
        .arg("--compute-csv-file")
        .arg(&compute_file)
        .arg("--simulation-seed")
        .arg(seed.to_string())
        .arg("--quiet"); // Disable logging for Monte Carlo subprocess
    
    // For boolean flags in clap, only add the flag if true (don't pass value)
    if config.enable_arbitrageurs {
        cmd.arg("--enable-arbitrageurs");
    }
    
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output()?;

    // Check stderr even on success - may contain warnings or errors
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() && (stderr.contains("error") || stderr.contains("ERROR") || stderr.contains("WARN")) {
        error!("[seed {}] stderr: {}", seed, stderr.trim());
    }

    if !output.status.success() {
        anyhow::bail!("Simulation failed with exit code {:?}: {}", output.status.code(), stderr);
    }

    // Verify output files were created and have reasonable size
    match std::fs::metadata(&output_file) {
        Ok(metadata) => {
            let size = metadata.len();
            if size < 100 {
                error!("[seed {}] Warning: Output CSV is suspiciously small ({} bytes)", seed, size);
            }
        }
        Err(e) => {
            error!("[seed {}] Warning: Output CSV not found: {}", seed, e);
        }
    }

    Ok(())
}

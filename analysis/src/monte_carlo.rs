use anyhow::{Context, Result};
use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use crate::{BASE_DIRS, SCENARIO_TYPES, aggregation::RunMetrics, loader, metrics};

/// Structure representing a scenario configuration to process
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    pub user_count: u32,
    pub scenario_type: String,  // "single", "single_no_arb", "dual", "dual_no_arb"
    pub base_path: PathBuf,
}

impl ScenarioConfig {
    pub fn scenario_name(&self) -> String {
        format!("{}_n{}", self.scenario_type, self.user_count)
    }
}

/// Discover all scenarios in the given base directory
pub fn discover_scenarios(base_dir: &Path) -> Result<Vec<ScenarioConfig>> {
    let mut scenarios = Vec::new();
        
    for user_count in BASE_DIRS {
        for scenario_type in SCENARIO_TYPES {
            let scenario_path = base_dir.join(user_count.to_string()).join(scenario_type);
            
            // Check if the folder exists and has CSV files
            if scenario_path.exists() && scenario_path.is_dir() {
                let pattern = scenario_path.join("seed_*.csv").to_string_lossy().to_string();
                if let Ok(mut entries) = glob(&pattern) {
                    if entries.next().is_some() {
                        scenarios.push(ScenarioConfig {
                            user_count,
                            scenario_type: scenario_type.to_string(),
                            base_path: scenario_path,
                        });
                        println!("  Discovered scenario: {} ({})", scenario_type, user_count);
                    }
                }
            }
        }
    }
    
    if scenarios.is_empty() {
        anyhow::bail!("No scenarios found in {}", base_dir.display());
    }
    
    println!("\nTotal scenarios discovered: {}\n", scenarios.len());
    Ok(scenarios)
}

/// Process a single scenario (all 1000 seed_*.csv files)
pub fn process_scenario(config: &ScenarioConfig) -> Result<Vec<RunMetrics>> {
    println!("\n{}", "=".repeat(60));
    println!("Processing scenario: {}", config.scenario_name());
    println!("{}", "=".repeat(60));
    
    // Find all seed_*.csv files (reserves)
    let pattern = config.base_path.join("seed_*.csv").to_string_lossy().to_string();
    let mut csv_files: Vec<PathBuf> = glob(&pattern)?
        .filter_map(Result::ok)
        .filter(|p| !p.to_string_lossy().contains("_compute"))
        .collect();
    
    csv_files.sort();
    
    println!("Found {} simulation files", csv_files.len());
    
    if csv_files.is_empty() {
        anyhow::bail!("No CSV files found for scenario {}", config.scenario_name());
    }
    
    // Progress bar
    let pb = ProgressBar::new(csv_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-")
    );
    
    // Parallel processing with Rayon
    let results: Vec<Result<RunMetrics>> = csv_files
        .par_iter()
        .map(|csv_path| {
            let result = process_single_run(csv_path);
            pb.inc(1);
            result
        })
        .collect();
    
    pb.finish_with_message("Completed!");
    
    // Collect successful results and display error details
    let mut metrics = Vec::new();
    let mut failed_files = Vec::new();
    
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(m) => metrics.push(m),
            Err(e) => {
                let file_path = &csv_files[i];
                eprintln!("❌ Error processing {}: {}", file_path.display(), e);
                failed_files.push((file_path.clone(), e));
            }
        }
    }
    
    println!("\nSuccessfully processed: {}/{}", metrics.len(), csv_files.len());
    if !failed_files.is_empty() {
        println!("   Failed: {} files", failed_files.len());
        for (path, err) in &failed_files {
            println!("  - {}: {}", path.display(), err);
        }
    }
    
    Ok(metrics)
}

/// Process a single run (one seed_XXX.csv file)
fn process_single_run(reserves_path: &Path) -> Result<RunMetrics> {
    // Extract seed number from filename
    let seed = extract_seed_from_filename(reserves_path)?;
    
    // Find corresponding compute file
    let compute_path = reserves_path.to_string_lossy().replace(".csv", "_compute.csv");
    let compute_path = PathBuf::from(compute_path);
    
    // Load data
    let reserves = loader::load_reserves_csv(reserves_path)
        .with_context(|| format!("Failed to load reserves CSV: {}", reserves_path.display()))?;
    
    let compute = if compute_path.exists() {
        loader::load_compute_csv(&compute_path)
            .with_context(|| format!("Failed to load compute CSV: {}", compute_path.display()))?
    } else {
        Vec::new()  // No compute data
    };
    
    // Calculate metrics using the new implementation
    let metrics = metrics::calculate_run_metrics(&reserves, &compute, seed)?;
    
    Ok(metrics)
}

/// Extract seed number from filename (e.g., "seed_1234.csv" -> 1234)
fn extract_seed_from_filename(path: &Path) -> Result<u64> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
    
    // Format: seed_XXX.csv
    let seed_str = filename
        .strip_prefix("seed_")
        .and_then(|s| s.strip_suffix(".csv"))
        .ok_or_else(|| anyhow::anyhow!("Filename doesn't match pattern seed_XXX.csv"))?;
    
    seed_str.parse::<u64>()
        .context("Failed to parse seed number")
}

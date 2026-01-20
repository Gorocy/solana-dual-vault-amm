mod aggregation;
mod export;
mod loader;
mod max_tps;
mod metrics;
mod monte_carlo;
mod types;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::max_tps::run_max_tps_analysis;

const LAMPORTS_PER_SOL_F64: f64 = solana_sdk::native_token::LAMPORTS_PER_SOL as f64;
const BASE_DIRS: [u32; 3] = [32, 128, 512];
const SCENARIO_TYPES: [&str; 4] = ["single", "single_no_arb", "dual", "dual_no_arb"];

#[derive(Parser, Debug)]
#[command(name = "analysis")]
#[command(about = "Analyze CPMM simulation results - Monte Carlo batch processing", long_about = None)]
pub struct AnalysisArgs {
    /// Mode: "single" for one simulation, "batch" for old batch, "monte-carlo" for 150GB processing
    #[arg(short, long, default_value = "monte-carlo")]
    mode: String,

    /// Base directory for monte-carlo mode (e.g., "." containing 32/, 128/, 512/)
    #[arg(short, long, default_value = ".")]
    base_dir: PathBuf,

    /// Path to reserves CSV file (for single mode)
    #[arg(short, long)]
    reserves: Option<PathBuf>,

    /// Path to compute CSV file (for single mode)
    #[arg(short, long)]
    compute: Option<PathBuf>,

    /// Scenario name (for single mode)
    #[arg(short, long)]
    scenario: Option<String>,

    /// User count configuration (for single mode)
    #[arg(short, long)]
    users: Option<u32>,

    /// Output directory for results CSV files
    #[arg(short, long, default_value = "final_output")]
    output: PathBuf,

    /// Export detailed per-run CSV files (large!)
    #[arg(long)]
    detailed: bool,

    /// Generate visualization charts (PNG images)
    #[arg(long)]
    charts: bool,

    /// Output directory for charts
    #[arg(long, default_value = "charts")]
    charts_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>>  {
    let args = AnalysisArgs::parse();

    match args.mode.as_str() {
        "monte-carlo" => run_monte_carlo_analysis(&args),
        "max-tps" => run_max_tps_analysis(&args),
        _ => {
            eprintln!(
                "Unknown mode: {}. Currently only 'monte-carlo' and 'max-tps' modes are supported",
                args.mode
            );
            eprintln!("Usage: cargo run --release -- --mode monte-carlo --base-dir . --output final_output");
            std::process::exit(1);
        }
    }
}

fn run_monte_carlo_analysis(args: &AnalysisArgs) -> Result<(), Box<dyn std::error::Error>>  {
    println!("\n{}", "=".repeat(80));
    println!("CPMM MONTE CARLO ANALYSIS - BATCH PROCESSING 150GB");
    println!("{}", "=".repeat(80));
    println!("Base directory: {}", args.base_dir.display());
    println!("Output directory: {}", args.output.display());
    println!("{}\n", "=".repeat(80));

    // Krok 1: Odkryj wszystkie scenariusze
    println!("Step 1: Discovering scenarios...");
    let scenarios = monte_carlo::discover_scenarios(&args.base_dir)?;

    // Krok 2: Przetwórz każdy scenariusz równolegle
    println!("\nStep 2: Processing scenarios with parallel batch processing...\n");

    let mut all_aggregations = Vec::new();
    let mut all_histograms = Vec::new();

    for scenario in &scenarios {
        // Przetwórz wszystkie ~1000 plików w scenariuszu
        let run_metrics = monte_carlo::process_scenario(scenario)?;

        // Agreguj wyniki
        let aggregation = aggregation::ScenarioAggregation::from_runs(
            &run_metrics,
            scenario.scenario_name(),
            scenario.user_count,
        );

        // Wygeneruj histogramy dla kluczowych metryk
        let histograms = export::generate_histograms(
            &run_metrics,
            &scenario.scenario_name(),
            scenario.user_count,
        );

        // Opcjonalnie: eksportuj szczegółowe wyniki per-run
        if args.detailed {
            export::export_detailed_runs(&run_metrics, &scenario.scenario_name(), &args.output)?;
        }

        all_aggregations.push(aggregation.clone());
        all_histograms.extend(histograms);

        // Wyświetl podsumowanie scenariusza
        print_scenario_summary(&aggregation);
    }

    // Krok 3: Eksportuj wyniki
    println!("\nStep 3: Exporting results...");
    export::export_results(&all_aggregations, &all_histograms, &args.output)?;

    // Krok 4: Podsumowanie końcowe
    print_final_summary(&all_aggregations);

    println!("\n{}", "=".repeat(80));
    println!("ANALYSIS COMPLETE!");
    println!("{}", "=".repeat(80));
    println!("Results saved to: {}", args.output.display());
    println!("- final_summary.csv (12 scenarios with aggregated statistics)");
    println!("- histograms.csv (distribution data for visualization)");
    if args.detailed {
        println!("- *_detailed.csv (per-simulation details)");
    }
    println!("{}\n", "=".repeat(80));

    Ok(())
}

fn print_scenario_summary(agg: &aggregation::ScenarioAggregation) {
    println!("\n{}", "-".repeat(70));
    println!("Scenario: {} (n={})", agg.scenario_name, agg.user_count);
    println!("{}", "-".repeat(70));
    println!("Total runs analyzed: {}", agg.total_runs);

    println!("\nWealth Distribution:");
    println!(
        "  User PnL:           Mean={:.2}, StdDev={:.2}, P95={:.2}",
        agg.user_pnl.mean, agg.user_pnl.std_dev, agg.user_pnl.p95
    );
    println!(
        "  Arb Profit:         Mean={:.2}, StdDev={:.2}, P95={:.2}",
        agg.arb_profit.mean, agg.arb_profit.std_dev, agg.arb_profit.p95
    );
    println!(
        "  LP Profit Value:    Mean={:.2}, StdDev={:.2}, P95={:.2}",
        agg.lp_profit_value.mean, agg.lp_profit_value.std_dev, agg.lp_profit_value.p95
    );

    println!("\nVolume & Efficiency:");
    println!(
        "  Total Volume:       Mean={:.2}, StdDev={:.2}, P95={:.2}",
        agg.total_volume.mean, agg.total_volume.std_dev, agg.total_volume.p95
    );
    println!(
        "  LP Profit/1M Vol:   Mean={:.2}, StdDev={:.2}, P95={:.2}",
        agg.lp_profit_per_1m_vol.mean,
        agg.lp_profit_per_1m_vol.std_dev,
        agg.lp_profit_per_1m_vol.p95
    );
    println!(
        "  Arb Trade Count:    Mean={:.0}, StdDev={:.0}, P95={:.0}",
        agg.arb_trade_count.mean, agg.arb_trade_count.std_dev, agg.arb_trade_count.p95
    );
    println!(
        "  Intervention Rate:  Mean={:.4}, StdDev={:.4}, P95={:.4}",
        agg.intervention_rate.mean, agg.intervention_rate.std_dev, agg.intervention_rate.p95
    );

    println!("\nMarket Metrics:");
    println!(
        "  Avg Spread %:       Mean={:.4}%, StdDev={:.4}, P95={:.4}",
        agg.avg_spread_pct.mean, agg.avg_spread_pct.std_dev, agg.avg_spread_pct.p95
    );
    println!(
        "  Max Spread %:       Mean={:.4}%, StdDev={:.4}, P95={:.4}",
        agg.max_spread_pct.mean, agg.max_spread_pct.std_dev, agg.max_spread_pct.p95
    );
    println!(
        "  RMSE Price:         Mean={:.4}, StdDev={:.4}, P95={:.4}",
        agg.rmse_price.mean, agg.rmse_price.std_dev, agg.rmse_price.p95
    );
    println!(
        "  IL %:               Mean={:.4}%, StdDev={:.4}, P95={:.4}",
        agg.impermanent_loss_pct.mean,
        agg.impermanent_loss_pct.std_dev,
        agg.impermanent_loss_pct.p95
    );

    println!("{}", "-".repeat(70));
}

fn print_final_summary(aggregations: &[aggregation::ScenarioAggregation]) {
    println!("\n{}", "=".repeat(90));
    println!("FINAL SUMMARY - ALL SCENARIOS");
    println!("{}", "=".repeat(90));
    println!(
        "{:<25} | {:>8} | {:>10} | {:>10} | {:>12} | {:>10}",
        "Scenario", "Runs", "User PnL", "Arb Profit", "LP Profit", "Avg Spread %"
    );
    println!("{}", "-".repeat(90));

    for agg in aggregations {
        println!(
            "{:<25} | {:>8} | {:>10.2} | {:>10.2} | {:>12.2} | {:>10.4}",
            agg.scenario_name,
            agg.total_runs,
            agg.user_pnl.mean,
            agg.arb_profit.mean,
            agg.lp_profit_value.mean,
            agg.avg_spread_pct.mean,
        );
    }
    println!("{}", "=".repeat(90));
}

use anyhow::Result;
use csv::Writer;
use std::path::Path;
use crate::aggregation::{Histogram, RunMetrics, ScenarioAggregation};

/// Export results to CSV files
pub fn export_results(
    aggregations: &[ScenarioAggregation],
    all_histograms: &[Histogram],
    output_dir: &Path,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    
    // 1. Export final_summary.csv (12 scenarios with aggregated statistics)
    export_final_summary(aggregations, output_dir)?;
    
    // 2. Export histograms.csv (data for density plots)
    export_histograms(all_histograms, output_dir)?;
    
    println!("\n  Results exported to: {}", output_dir.display());
    println!("  - final_summary.csv: {} scenarios", aggregations.len());
    println!("  - histograms.csv: {} histogram distributions", all_histograms.len());
    
    Ok(())
}

/// Export final_summary.csv with aggregated statistics (12 rows)
fn export_final_summary(aggregations: &[ScenarioAggregation], output_dir: &Path) -> Result<()> {
    let path = output_dir.join("final_summary.csv");
    let mut wtr = Writer::from_path(&path)?;
    
    // Header - all metrics with Mean, StdDev, P95
    wtr.write_record(&[
        "scenario",
        "user_count",
        "total_runs",
        // Wealth Distribution
        "user_pnl_mean",
        "user_pnl_stddev",
        "user_pnl_p95",
        "arb_profit_mean",
        "arb_profit_stddev",
        "arb_profit_p95",
        "lp_profit_value_mean",
        "lp_profit_value_stddev",
        "lp_profit_value_p95",
        // Volume & Efficiency
        "total_volume_mean",
        "total_volume_stddev",
        "total_volume_p95",
        "lp_profit_per_1m_vol_mean",
        "lp_profit_per_1m_vol_stddev",
        "lp_profit_per_1m_vol_p95",
        "arb_trade_count_mean",
        "arb_trade_count_stddev",
        "arb_trade_count_p95",
        "intervention_rate_mean",
        "intervention_rate_stddev",
        "intervention_rate_p95",
        // Market Metrics
        "avg_spread_pct_mean",
        "avg_spread_pct_stddev",
        "avg_spread_pct_p95",
        "max_spread_pct_mean",
        "max_spread_pct_stddev",
        "max_spread_pct_p95",
        "rmse_price_mean",
        "rmse_price_stddev",
        "rmse_price_p95",
        "impermanent_loss_pct_mean",
        "impermanent_loss_pct_stddev",
        "impermanent_loss_pct_p95",
        // Compute
        "total_compute_units_mean",
        "total_compute_units_stddev",
        "total_compute_units_p95",
        "avg_cu_per_swap_mean",
        "avg_cu_per_swap_stddev",
        "avg_cu_per_swap_p95",
    ])?;
    
    // Data - each aggregation is one row (1 scenario)
    for agg in aggregations {
        wtr.write_record(&[
            &agg.scenario_name,
            &agg.user_count.to_string(),
            &agg.total_runs.to_string(),
            // Wealth Distribution
            &format!("{:.2}", agg.user_pnl.mean),
            &format!("{:.2}", agg.user_pnl.std_dev),
            &format!("{:.2}", agg.user_pnl.p95),
            &format!("{:.2}", agg.arb_profit.mean),
            &format!("{:.2}", agg.arb_profit.std_dev),
            &format!("{:.2}", agg.arb_profit.p95),
            &format!("{:.2}", agg.lp_profit_value.mean),
            &format!("{:.2}", agg.lp_profit_value.std_dev),
            &format!("{:.2}", agg.lp_profit_value.p95),
            // Volume & Efficiency
            &format!("{:.2}", agg.total_volume.mean),
            &format!("{:.2}", agg.total_volume.std_dev),
            &format!("{:.2}", agg.total_volume.p95),
            &format!("{:.2}", agg.lp_profit_per_1m_vol.mean),
            &format!("{:.2}", agg.lp_profit_per_1m_vol.std_dev),
            &format!("{:.2}", agg.lp_profit_per_1m_vol.p95),
            &format!("{:.2}", agg.arb_trade_count.mean),
            &format!("{:.2}", agg.arb_trade_count.std_dev),
            &format!("{:.2}", agg.arb_trade_count.p95),
            &format!("{:.4}", agg.intervention_rate.mean),
            &format!("{:.4}", agg.intervention_rate.std_dev),
            &format!("{:.4}", agg.intervention_rate.p95),
            // Market Metrics
            &format!("{:.4}", agg.avg_spread_pct.mean),
            &format!("{:.4}", agg.avg_spread_pct.std_dev),
            &format!("{:.4}", agg.avg_spread_pct.p95),
            &format!("{:.4}", agg.max_spread_pct.mean),
            &format!("{:.4}", agg.max_spread_pct.std_dev),
            &format!("{:.4}", agg.max_spread_pct.p95),
            &format!("{:.4}", agg.rmse_price.mean),
            &format!("{:.4}", agg.rmse_price.std_dev),
            &format!("{:.4}", agg.rmse_price.p95),
            &format!("{:.4}", agg.impermanent_loss_pct.mean),
            &format!("{:.4}", agg.impermanent_loss_pct.std_dev),
            &format!("{:.4}", agg.impermanent_loss_pct.p95),
            // Compute
            &format!("{:.0}", agg.total_compute_units.mean),
            &format!("{:.0}", agg.total_compute_units.std_dev),
            &format!("{:.0}", agg.total_compute_units.p95),
            &format!("{:.2}", agg.avg_cu_per_swap.mean),
            &format!("{:.2}", agg.avg_cu_per_swap.std_dev),
            &format!("{:.2}", agg.avg_cu_per_swap.p95),
        ])?;
    }
    
    wtr.flush()?;
    println!("  Exported: {}", path.display());
    Ok(())
}

/// Export histograms.csv with data for density plots
fn export_histograms(histograms: &[Histogram], output_dir: &Path) -> Result<()> {
    let path = output_dir.join("histograms.csv");
    let mut wtr = Writer::from_path(&path)?;
    
    // Header
    wtr.write_record(&[
        "metric_name",
        "scenario",
        "user_count",
        "bin_lower",
        "bin_upper",
        "count",
        "frequency_pct",
    ])?;
    
    // Data - each bin is one row
    for hist in histograms {
        for bin in &hist.bins {
            wtr.write_record(&[
                &hist.metric_name,
                &hist.scenario,
                &hist.user_count.to_string(),
                &format!("{:.4}", bin.lower_bound),
                &format!("{:.4}", bin.upper_bound),
                &bin.count.to_string(),
                &format!("{:.4}", bin.frequency),
            ])?;
        }
    }
    
    wtr.flush()?;
    println!("  Exported: {}", path.display());
    Ok(())
}

/// Generate histograms for key metrics
pub fn generate_histograms(
    runs: &[RunMetrics],
    scenario_name: &str,
    user_count: u32,
) -> Vec<Histogram> {
    let mut histograms = Vec::new();
    
    // User PnL histogram
    let user_pnl_values: Vec<f64> = runs.iter().map(|r| r.user_pnl).collect();
    histograms.push(Histogram::from_values(
        &user_pnl_values,
        "User_PnL".to_string(),
        scenario_name.to_string(),
        user_count,
        50,  // 50 bins
    ));
    
    // Avg Spread histogram
    let spread_values: Vec<f64> = runs.iter().map(|r| r.avg_spread_pct).collect();
    histograms.push(Histogram::from_values(
        &spread_values,
        "Avg_Spread_Pct".to_string(),
        scenario_name.to_string(),
        user_count,
        50,
    ));
    
    // Arb Profit histogram
    let arb_profit_values: Vec<f64> = runs.iter().map(|r| r.arb_profit).collect();
    histograms.push(Histogram::from_values(
        &arb_profit_values,
        "Arb_Profit".to_string(),
        scenario_name.to_string(),
        user_count,
        50,
    ));
    
    // LP Profit Value histogram
    let lp_profit_values: Vec<f64> = runs.iter().map(|r| r.lp_profit_value).collect();
    histograms.push(Histogram::from_values(
        &lp_profit_values,
        "LP_Profit_Value".to_string(),
        scenario_name.to_string(),
        user_count,
        50,
    ));
    
    histograms
}

/// Optionally: Export detailed per-run data (can be large!)

pub fn export_detailed_runs(
    runs: &[RunMetrics],
    scenario_name: &str,
    output_dir: &Path,
) -> Result<()> {
    let path: std::path::PathBuf = output_dir.join(format!("{}_detailed.csv", scenario_name));
    let mut wtr = Writer::from_path(&path)?;
    
    // Nagłówek
    wtr.write_record(&[
        "seed",
        "user_pnl",
        "arb_profit",
        "lp_profit_value",
        "total_volume",
        "lp_profit_per_1m_vol",
        "arb_trade_count",
        "intervention_rate",
        "avg_spread_pct",
        "max_spread_pct",
        "rmse_price",
        "impermanent_loss_pct",
        "total_compute_units",
        "avg_cu_per_swap",
        "total_swaps",
        "successful_swaps",
    ])?;
    
    // Dane
    for run in runs {
        wtr.write_record(&[
            &run.seed.to_string(),
            &format!("{:.4}", run.user_pnl),
            &format!("{:.4}", run.arb_profit),
            &format!("{:.4}", run.lp_profit_value),
            &format!("{:.4}", run.total_volume),
            &format!("{:.4}", run.lp_profit_per_1m_vol),
            &run.arb_trade_count.to_string(),
            &format!("{:.6}", run.intervention_rate),
            &format!("{:.6}", run.avg_spread_pct),
            &format!("{:.6}", run.max_spread_pct),
            &format!("{:.6}", run.rmse_price),
            &format!("{:.6}", run.impermanent_loss_pct),
            &run.total_compute_units.to_string(),
            &format!("{:.2}", run.avg_cu_per_swap),
            &run.total_swaps.to_string(),
            &run.successful_swaps.to_string(),
        ])?;
    }
    
    wtr.flush()?;
    Ok(())
}

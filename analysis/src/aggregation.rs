use serde::{Deserialize, Serialize};

/// Statistic agregation for a given metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedStats {
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub p95: f64,  // 95-ty percentyl (VaR 95%)
    pub p99: f64,  // 99-ty percentyl (VaR 99%)
    pub median: f64,
    pub count: usize,
}

impl AggregatedStats {
    /// Create AggregatedStats from a slice of values
    pub fn from_values(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }

        let mut sorted = values.to_vec();
        // Filter out NaN values before sorting
        sorted.retain(|x| x.is_finite());
        if sorted.is_empty() {
            return None;
        }
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
        let variance = sorted.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / sorted.len() as f64;
        let std_dev = variance.sqrt();
        
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };
        
        let p95_idx = ((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1);
        let p99_idx = ((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1);
        
        Some(AggregatedStats {
            mean,
            std_dev,
            min,
            max,
            median,
            p95: sorted[p95_idx],
            p99: sorted[p99_idx],
            count: values.len(),
        })
    }
}

/// Structure holding the results of a single simulation (per run)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetrics {
    pub seed: u64,
    
    // Wealth Distribution
    pub user_pnl: f64,              // Change in user portfolio value (Token B)
    pub arb_profit: f64,             // Total arbitrageur profit
    pub lp_profit_value: f64,        // Nominal LP profit (in Token B)
    
    // Volume & Efficiency
    pub total_volume: f64,           // Sum of amount_out from successful transactions
    pub lp_profit_per_1m_vol: f64,   // LP_Profit_Value / Total_Volume * 1M
    pub arb_trade_count: u64,        // Number of arbitrage trades
    pub intervention_rate: f64,      // Ratio of arbitrage transactions to all transactions
    
    // Standard Market Metrics
    pub avg_spread_pct: f64,         // Average spread in %
    pub max_spread_pct: f64,         // Maximum spread in %
    pub rmse_price: f64,             // RMSE of price from market price
    pub impermanent_loss_pct: f64,   // Impermanent Loss in %
    
    // Compute metrics
    pub total_compute_units: u64,
    pub avg_cu_per_swap: f64,
    pub total_swaps: u64,
    pub successful_swaps: u64,
}

/// Aggregated results for an entire scenario (1000 simulations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioAggregation {
    pub scenario_name: String,
    pub user_count: u32,
    pub total_runs: usize,
    
    // Wealth Distribution
    pub user_pnl: AggregatedStats,
    pub arb_profit: AggregatedStats,
    pub lp_profit_value: AggregatedStats,
    
    // Volume & Efficiency
    pub total_volume: AggregatedStats,
    pub lp_profit_per_1m_vol: AggregatedStats,
    pub arb_trade_count: AggregatedStats,
    pub intervention_rate: AggregatedStats,
    
    // Market Metrics
    pub avg_spread_pct: AggregatedStats,
    pub max_spread_pct: AggregatedStats,
    pub rmse_price: AggregatedStats,
    pub impermanent_loss_pct: AggregatedStats,
    
    // Compute
    pub total_compute_units: AggregatedStats,
    pub avg_cu_per_swap: AggregatedStats,
}

impl ScenarioAggregation {
    /// Aggregate results from multiple simulations
    pub fn from_runs(runs: &[RunMetrics], scenario_name: String, user_count: u32) -> Self {
        let user_pnl = AggregatedStats::from_values(
            &runs.iter().map(|r| r.user_pnl).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let arb_profit = AggregatedStats::from_values(
            &runs.iter().map(|r| r.arb_profit).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let lp_profit_value = AggregatedStats::from_values(
            &runs.iter().map(|r| r.lp_profit_value).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let total_volume = AggregatedStats::from_values(
            &runs.iter().map(|r| r.total_volume).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let lp_profit_per_1m_vol = AggregatedStats::from_values(
            &runs.iter().map(|r| r.lp_profit_per_1m_vol).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let arb_trade_count = AggregatedStats::from_values(
            &runs.iter().map(|r| r.arb_trade_count as f64).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let intervention_rate = AggregatedStats::from_values(
            &runs.iter().map(|r| r.intervention_rate).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let avg_spread_pct = AggregatedStats::from_values(
            &runs.iter().map(|r| r.avg_spread_pct).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let max_spread_pct = AggregatedStats::from_values(
            &runs.iter().map(|r| r.max_spread_pct).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let rmse_price = AggregatedStats::from_values(
            &runs.iter().map(|r| r.rmse_price).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let impermanent_loss_pct = AggregatedStats::from_values(
            &runs.iter().map(|r| r.impermanent_loss_pct).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let total_compute_units = AggregatedStats::from_values(
            &runs.iter().map(|r| r.total_compute_units as f64).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        let avg_cu_per_swap = AggregatedStats::from_values(
            &runs.iter().map(|r| r.avg_cu_per_swap).collect::<Vec<_>>()
        ).unwrap_or_default();
        
        ScenarioAggregation {
            scenario_name,
            user_count,
            total_runs: runs.len(),
            user_pnl,
            arb_profit,
            lp_profit_value,
            total_volume,
            lp_profit_per_1m_vol,
            arb_trade_count,
            intervention_rate,
            avg_spread_pct,
            max_spread_pct,
            rmse_price,
            impermanent_loss_pct,
            total_compute_units,
            avg_cu_per_swap,
        }
    }
}

impl Default for AggregatedStats {
    fn default() -> Self {
        Self {
            mean: 0.0,
            std_dev: 0.0,
            min: 0.0,
            max: 0.0,
            p95: 0.0,
            p99: 0.0,
            median: 0.0,
            count: 0,
        }
    }
}

/// Histogram dla wizualizacji rozkładu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub metric_name: String,
    pub scenario: String,
    pub user_count: u32,
    pub bins: Vec<HistogramBin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBin {
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub count: usize,
    pub frequency: f64,  // jako % całości
}

impl Histogram {
    /// Utwórz histogram z danych (100 koszyków)
    pub fn from_values(
        values: &[f64],
        metric_name: String,
        scenario: String,
        user_count: u32,
        num_bins: usize,
    ) -> Self {
        if values.is_empty() {
            return Histogram {
                metric_name,
                scenario,
                user_count,
                bins: Vec::new(),
            };
        }
        
        // Filter finite values only
        let finite_values: Vec<f64> = values.iter().cloned().filter(|x| x.is_finite()).collect();
        if finite_values.is_empty() {
            return Histogram {
                metric_name,
                scenario,
                user_count,
                bins: Vec::new(),
            };
        }
        
        let min = finite_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = finite_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        let mut bins = Vec::with_capacity(num_bins);
        
        if min == max {
            // All values are the same - create single bin
            bins.push(HistogramBin {
                lower_bound: min,
                upper_bound: max,
                count: finite_values.len(),
                frequency: 100.0,
            });
            return Histogram {
                metric_name,
                scenario,
                user_count,
                bins,
            };
        }
        
        let bin_width = (max - min) / num_bins as f64;
        
        let mut bins = Vec::with_capacity(num_bins);
        for i in 0..num_bins {
            let lower = min + i as f64 * bin_width;
            let upper = min + (i + 1) as f64 * bin_width;
            
            let count = finite_values.iter().filter(|&&v| {
                v >= lower && (v < upper || (i == num_bins - 1 && v <= upper))
            }).count();
            
            bins.push(HistogramBin {
                lower_bound: lower,
                upper_bound: upper,
                count,
                frequency: count as f64 / finite_values.len() as f64 * 100.0,
            });
        }
        
        Histogram {
            metric_name,
            scenario,
            user_count,
            bins,
        }
    }
}

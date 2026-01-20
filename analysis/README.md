# Analysis Tool

Statistical analysis pipeline for processing Monte Carlo simulation results. Aggregates thousands of CSV files in parallel using Rust, Polars (DataFrame library), and Rayon (parallelism framework) to generate comprehensive statistics across 12 experimental scenarios.

## Overview

This tool processes the output from ~12,000 simulation runs (1000 per scenario × 12 scenarios) to produce aggregated statistics, distribution histograms, and summary tables for research analysis. It is optimized for high-throughput batch processing of large datasets (~150 GB total).

## Quick Start

### Requirements

- Rust 1.70+
- RAM: 4-8 GB (for ~1000 files per scenario)
- CPU: Multi-core (Rayon utilizes all cores)

### Directory Structure

Program expects the following folder structure:

```
./
├── 32/          # Scenario with 32 users
│   ├── dual/
│   │   ├── seed_1000.csv
│   │   ├── seed_1000_compute.csv
│   │   ├── seed_1001.csv
│   │   ├── seed_1001_compute.csv
│   │   └── ... (1000 pairs of files)
│   ├── dual_no_arb/
│   ├── single/
│   └── single_no_arb/
├── 128/         # Scenario with 128 users
│   ├── dual/
│   ├── dual_no_arb/
│   ├── single/
│   └── single_no_arb/
└── 512/         # Scenario with 512 users
    ├── dual/
    ├── dual_no_arb/
    ├── single/
    └── single_no_arb/
```

Each `seed_<N>.csv` file represents one Monte Carlo simulation (1 run = 2000 slots).

### Execution

```bash
cd analysis
cargo build --release

# Analyze all scenarios (default mode: monte-carlo)
cargo run --release -- --base-dir ../ --output ./final_output
```

**Parameters:**
- `--base-dir` - Root directory containing 32/, 128/, 512/ folders (default: `.`)
- `--output` - Where to save output CSV files (default: `final_output`)
- `--detailed` - Export per-run detailed data (optional, generates large files)
- `--charts` - Generate PNG charts (optional, requires visualization features)
- `--charts-dir` - Directory for charts (default: `charts`)

## Processing Pipeline

### Step 1: Scenario Discovery

Scans directories 32/, 128/, 512/ and identifies 12 combinations:
- 3 load levels: 32, 128, 512 users
- 2 architectures: `single` (Single Vault), `dual` (Dual Vault)
- 2 market conditions: with arbitrage, without arbitrage (`_no_arb`)

### Step 2: Parallel File Processing

For each scenario:
- Loads ~1000 pairs of files (`seed_N.csv` + `seed_N_compute.csv`)
- Utilizes all CPU cores (Rayon)
- Displays real-time progress (progress bar)

### Step 3: Per-Run Metrics Calculation

For each individual simulation:
- **Price stability:** average spread, maximum spread, RMSE of price
- **LP economics:** initial and final TVL, LP profit (value and %), Impermanent Loss (%)
- **Arbitrage:** total arbitrageur profit, number of arbitrage transactions, intervention rate
- **Volume:** total transaction volume, LP profit per 1M volume
- **Efficiency:** total Compute Units consumed, average CU per swap

### Step 4: Statistical Aggregation

For each of 12 scenarios:
- **Mean** (arithmetic average)
- **StdDev** (standard deviation)
- **Min / Max** (extreme values)
- **P95** (95th percentile)
- **P99** (99th percentile) - for histograms

### Step 5: Histogram Generation

Probability distributions (100 bins) for key metrics:
- Price spread (avg_spread_pct, max_spread_pct)
- Economic outcomes (arb_profit, lp_profit_value)
- Efficiency (total_compute_units)

## Output Files

Program generates 2 CSV files in the output directory:

### 1. final_summary.csv - Aggregated Statistics

12 rows (one per scenario) × 41 columns (metrics with mean/stddev/p95):

```csv
scenario,user_count,total_runs,user_pnl_mean,user_pnl_stddev,user_pnl_p95,arb_profit_mean,...
single_n32,32,1000,-372740.95,27253.02,-326767.81,165941.25,...
dual_n32,32,1000,-371597.43,27103.97,-325701.34,73541.41,...
single_no_arb_n32,32,1000,-184617.43,11630.38,-165964.12,0.00,...
dual_no_arb_n32,32,1000,-350259.65,25545.01,-306755.18,0.00,...
...
```

**Columns (41 total):**
- `scenario` - Scenario name (e.g., `single_n32`, `dual_n128`)
- `user_count` - Number of users (32, 128, 512)
- `total_runs` - Number of analyzed simulations (~1000)
- For each metric: `_mean`, `_stddev`, `_p95`

**Metrics:**
- `user_pnl` - User financial outcome (User Profit & Loss)
- `arb_profit` - Arbitrageur profit
- `lp_profit_value` - LP profit (absolute value)
- `total_volume` - Total transaction volume
- `lp_profit_per_1m_vol` - LP profit per 1M volume units
- `arb_trade_count` - Number of arbitrage transactions
- `intervention_rate` - Arbitrage intervention rate
- `avg_spread_pct` - Average price spread (%)
- `max_spread_pct` - Maximum price spread (%)
- `rmse_price` - Root Mean Square Error of price
- `impermanent_loss_pct` - Impermanent Loss (%)
- `total_compute_units` - Total CU consumption
- `avg_cu_per_swap` - Average CU per transaction

### 2. histograms.csv - Probability Distributions

Visualization data for distribution analysis:

```csv
scenario,metric_name,bin_start,bin_end,count,frequency_pct
single_n32,avg_spread_pct,0.0,0.1,0,0.0
single_n32,avg_spread_pct,0.1,0.2,5,0.5
single_n32,avg_spread_pct,0.2,0.3,12,1.2
...
```

- 100 bins per metric
- Supported metrics: `avg_spread_pct`, `max_spread_pct`, `arb_profit`, `lp_profit_value`, `total_compute_units`

## Input File Format

### seed_<N>.csv (Reserves - vault state)

```csv
slot,market_price,vault_0_price,vault_0_reserve_a,vault_0_reserve_b,vault_1_price,...,arb_balance_a,arb_balance_b,user_0_balance_a,user_0_balance_b,...
0,1.0,1.0,1000000000,1000000000,1.0,...,5000000000,5000000000,10000000,10000000,...
1,1.001,1.0001,999900000,1000100000,1.0002,...,5000100000,4999900000,9990000,10010000,...
...
```

**Columns:**
- `slot` - Time slot number (0-1999)
- `market_price` - External market price (from GBM model)
- `vault_N_price` - Price in N-th vault
- `vault_N_reserve_a/b` - Token A/B reserves in N-th vault
- `arb_balance_a/b` - Arbitrageur balances
- `user_N_balance_a/b` - User balances

### seed_<N>_compute.csv (Compute - computational units)

```csv
slot,compute_units,success,amount_in
0,5000,true,1000000
1,5100,true,1500000
2,5200,false,2000000
...
```

**Columns:**
- `slot` - Slot number
- `compute_units` - Consumed Compute Units (CU)
- `success` - Whether transaction succeeded
- `amount_in` - Input transaction amount (in lamports)

## Metrics Details

### A. Price Stability

- **avg_spread_pct** - Average price spread: `(max_price - min_price) / avg_price * 100`
- **max_spread_pct** - Maximum spread across entire simulation
- **rmse_price** - Root Mean Square Error: `sqrt(mean((vault_price - market_price)^2))`
  - Measures how well vaults track external market price

### B. LP Economics (Liquidity Provider)

- **lp_profit_value** - Absolute LP profit value in lamports
- **lp_profit_per_1m_vol** - Normalized profit: `lp_profit / (total_volume / 1_000_000)`
- **impermanent_loss_pct** - IL relative to HODL strategy: `((final_value / initial_value) / price_ratio - 1) * 100`

### C. Arbitrage

- **arb_profit** - Total profit of all arbitrageurs (in lamports)
- **arb_trade_count** - Number of transactions executed by arbitrageurs
- **intervention_rate** - Intervention frequency: `arb_trades / total_slots`

### D. Volume and Activity

- **total_volume** - Sum of amount_in for all transactions (in lamports)
- **user_pnl** - User financial outcome: `sum(final_balances - initial_balances)`

### E. Computational Efficiency

- **total_compute_units** - Sum of CU for all transactions
- **avg_cu_per_swap** - Average CU per transaction: `total_cu / transaction_count`

## Performance

Program utilizes:
- **Polars** - Fast DataFrame processing library (similar to Pandas, but in Rust)
- **Rayon** - Parallel processing across all CPU cores
- **Progress bars** - Real-time progress visualization (indicatif)

**Typical processing time:**
- ~1000 files per scenario × 12 scenarios = ~12,000 files
- Time: **1-3 minutes** (depending on CPU and disk)
- RAM: **2-4 GB** (Polars loads data lazily - LazyFrame)

## Usage Examples

### Standard analysis (all scenarios)

```bash
cargo run --release
```

### Analysis with custom data path

```bash
cargo run --release -- --base-dir /path/to/simulation/results --output ./my_results
```

### Analysis with detailed data export

```bash
cargo run --release -- --detailed --output ./detailed_results
```

Note: `--detailed` mode generates additional CSV files with per-run metrics (~12 MB per scenario)

### Help and available options

```bash
cargo run --release -- --help
```

## For Research Applications

Program designed to generate data for academic/engineering work:

1. **final_summary.csv** → Comparative scenario tables (LaTeX)
2. **histograms.csv** → Probability density distribution charts
3. **Percentiles (P95, P99)** → Risk analysis (Value at Risk)
4. **Mean ± StdDev** → Confidence intervals for charts

**Example statistical analysis:**
- Comparison Single Vault vs Dual Vault
- Impact of user count (32 vs 128 vs 512)
- Effect of external arbitrage (with_arb vs no_arb)
- Trade-off: price stability vs computational cost (CU)

## Code Structure

```
analysis/src/
├── main.rs           # Entry point, CLI, orchestration
├── types.rs          # Data structure definitions (ScenarioConfig, RunMetrics)
├── loader.rs         # CSV loading (Polars LazyFrame)
├── metrics.rs        # Per-run metric calculations
├── aggregation.rs    # Statistical aggregation (mean, std, percentile)
├── export.rs         # Export results to CSV
├── monte_carlo.rs    # Batch processing logic
└── max_tps.rs        # Additional analysis mode (optional)
```

## Troubleshooting

### Problem: "No scenarios found"

**Solution:** Ensure directory structure is correct (32/, 128/, 512/ with subdirectories)

### Problem: "Failed to parse CSV"

**Solution:** Check seed_N.csv file format - must contain required columns (slot, market_price, vault_0_price, ...)

### Problem: Out of memory

**Solution:** Program uses LazyFrame (doesn't load all data into memory). If problem persists, reduce parallel thread count: `export RAYON_NUM_THREADS=4`

### Problem: Slow processing

**Solution:**
- Use SSD storage (not HDD) for input data
- Increase Rayon thread count (default = number of CPU cores)
- Compile in release mode: `cargo build --release`

## Related Documentation

- [../simulation/README.md](../simulation/README.md) - How simulation data is generated
- [../final_output/README.md](../final_output/README.md) - Output file specifications
- [../bigquery/README.md](../bigquery/README.md) - On-chain data validation

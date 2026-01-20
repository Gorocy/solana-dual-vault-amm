# Final Output

Aggregated statistical results from 12,000 Monte Carlo simulation runs. This directory contains summary tables, distribution histograms, and visualization notebooks used for research analysis and figure generation.

## Contents

### 1. Final Summary ([final_summary.csv](final_summary.csv))

Master table containing aggregated statistics for all 12 experimental scenarios. Each row represents one scenario (combination of architecture, user count, and market condition).

**Dimensions:** 12 rows × 41 columns

**Row structure:**
```csv
scenario,user_count,total_runs,<metric>_mean,<metric>_stddev,<metric>_p95,...
```

**Scenarios:**
- `single_n32`, `single_no_arb_n32`, `dual_n32`, `dual_no_arb_n32`
- `single_n128`, `single_no_arb_n128`, `dual_n128`, `dual_no_arb_n128`
- `single_n512`, `single_no_arb_n512`, `dual_n512`, `dual_no_arb_n512`

**Metrics categories:**

1. **User Economics** (3 metrics × 3 stats = 9 columns)
   - `user_pnl` - User profit/loss (negative = cost of trading)
   - Mean, standard deviation, 95th percentile

2. **Arbitrage Activity** (2 metrics × 3 stats = 6 columns)
   - `arb_profit` - Total arbitrageur profit
   - `arb_trade_count` - Number of arbitrage transactions
   - Mean, standard deviation, 95th percentile

3. **LP Economics** (2 metrics × 3 stats = 6 columns)
   - `lp_profit_value` - Absolute LP profit (lamports)
   - `lp_profit_per_1m_vol` - Normalized profit per 1M volume
   - Mean, standard deviation, 95th percentile

4. **Market Quality** (4 metrics × 3 stats = 12 columns)
   - `avg_spread_pct` - Average price spread across vaults
   - `max_spread_pct` - Maximum observed spread
   - `rmse_price` - Root mean square error vs external price
   - `intervention_rate` - Frequency of arbitrage interventions
   - Mean, standard deviation, 95th percentile

5. **Efficiency** (3 metrics × 3 stats = 9 columns)
   - `total_volume` - Aggregate swap volume
   - `total_compute_units` - Total CU consumption
   - `avg_cu_per_swap` - Average CU per transaction
   - Mean, standard deviation, 95th percentile

6. **LP Risk** (1 metric × 3 stats = 3 columns)
   - `impermanent_loss_pct` - IL relative to HODL strategy
   - Mean, standard deviation, 95th percentile

**Example row (abbreviated):**
```csv
dual_n128,128,1000,-1450328.70,53969.53,-1358033.67,331701.51,106116.50,...
```

This row represents:
- Scenario: Dual Vault, 128 users, with arbitrage
- 1000 simulation runs analyzed
- User PnL mean: -1.45M lamports (cost of trading fees + slippage)
- Arbitrage profit mean: 331k lamports
- ... (38 more statistics)

### 2. Histograms ([histograms.csv](histograms.csv))

Probability distribution data for key metrics across all scenarios. Each metric is binned into 100 equal-width intervals covering the observed range.

**Structure:**
```csv
scenario,metric_name,bin_start,bin_end,count,frequency_pct
```

**Supported metrics:**
- `avg_spread_pct` - Average spread distribution
- `max_spread_pct` - Maximum spread distribution
- `arb_profit` - Arbitrage profit distribution
- `lp_profit_value` - LP profit distribution
- `total_compute_units` - CU consumption distribution

**Example rows:**
```csv
dual_n128,avg_spread_pct,6.0,6.5,45,4.5
dual_n128,avg_spread_pct,6.5,7.0,123,12.3
dual_n128,avg_spread_pct,7.0,7.5,187,18.7
```

Interpretation: In the dual_n128 scenario:
- 45 simulations (4.5%) had average spread between 6.0% and 6.5%
- 123 simulations (12.3%) had spread between 6.5% and 7.0%
- 187 simulations (18.7%) had spread between 7.0% and 7.5%

**Purpose:** Enables visualization of distribution shapes (normal, skewed, bimodal) and tail risk analysis.

### 3. Figure Generation Notebook ([generate_figures.ipynb](generate_figures.ipynb))

Jupyter notebook for creating publication-quality figures from the aggregated data.

**Dependencies:**
- Python 3.8+
- pandas
- matplotlib
- seaborn (optional, for enhanced aesthetics)

**Generated figures:**

1. **Price Spread Comparison**
   - Bar chart comparing average spread across scenarios
   - Error bars showing standard deviation
   - Highlights Dual Vault advantage

2. **LP Profit per Volume**
   - Comparison of normalized LP returns
   - Demonstrates value capture from arbitrage reduction

3. **User Welfare (PnL)**
   - Cost of trading across architectures
   - Shows improved execution prices in Dual Vault

4. **Computational Overhead**
   - CU consumption per swap
   - Illustrates cost-benefit trade-off

5. **Distribution Plots**
   - Histograms and kernel density estimates
   - Percentile overlays (P95, P99)

**Execution:**
```bash
# Install dependencies
pip install jupyter pandas matplotlib

# Launch notebook
jupyter notebook generate_figures.ipynb
```

**Customization:** Edit cell parameters to:
- Change color schemes
- Adjust figure dimensions
- Filter specific scenarios
- Export different formats (PNG, PDF, SVG)

## Data Generation

These files are produced by the [analysis tool](../analysis/):

```bash
cd ../analysis
cargo run --release -- --base-dir ../ --output ../final_output
```

**Input:** ~12,000 CSV files (2 per simulation × 1000 runs × 12 scenarios)

**Processing time:** 2-3 minutes on modern hardware

**Output size:** 
- final_summary.csv: ~25 KB (highly compressed)
- histograms.csv: ~500 KB (100 bins × 5 metrics × 12 scenarios)

## Key Findings Summary

**Price Stability:**
- Dual Vault reduces spread by 54-79% vs Single Vault (no arbitrage scenarios)
- RMSE improvement of 23-68% in price tracking

**LP Economics:**
- LP profit +89% to +131% higher in Dual Vault (normalized by volume)
- Arbitrageur profit reduced by ~56% (value captured by LPs instead)

**User Welfare:**
- 0.3-2.9% better execution prices in Dual Vault (with arbitrage)
- Improvement scales with network congestion

**Computational Cost:**
- Dual Vault requires +91% more CU per swap (~59k vs ~31k)
- Remains well within Solana's 200k CU transaction limit
- Cost justified by economic benefits

**Impermanent Loss:**
- Higher IL in Dual Vault due to increased transaction volume
- Net positive for LPs: Higher IL offset by greater fee capture + internalized arbitrage value

See [final_summary.csv](final_summary.csv) for complete numerical results.

## Statistical Methodology

All statistics computed using standard formulas:

**Mean:**
```
μ = (1/N) Σ xᵢ
```

**Standard Deviation:**
```
σ = sqrt((1/N) Σ (xᵢ - μ)²)
```

**95th Percentile (P95):**
```
P95 = value at 95% cumulative distribution
```

**Interpretation:**
- Mean: Central tendency (expected value)
- StdDev: Variability / uncertainty
- P95: Risk metric (only 5% of runs exceed this value)

For skewed distributions (e.g., arbitrage profit), P95 is more informative than mean.

## Reproducibility

Results are deterministic given:
1. Same simulation seeds (1000-1999 per scenario)
2. Fixed RNG seed for GBM price generation
3. Identical smart contract bytecode
4. LiteSVM version consistency

To reproduce:
```bash
# Run simulations (generates raw CSVs)
cd ../simulation
for binary in dual_32 dual_no_arb_32 ...; do
  cargo run --release --bin $binary
done

# Aggregate results
cd ../analysis
cargo run --release

# Results should match final_summary.csv exactly
diff final_output/final_summary.csv ../final_output/final_summary.csv
```

**Expected:** Zero differences (bit-identical CSVs)

## Usage in Research

These files are referenced in:

1. **Academic papers:**
   - Tables: Direct import from final_summary.csv
   - Figures: Generated via generate_figures.ipynb
   - Confidence intervals: Mean ± StdDev

2. **Presentations:**
   - Bar charts for high-level comparisons
   - Distribution plots for detailed analysis

3. **Further analysis:**
   - Load CSVs into R, Python, Julia for custom statistics
   - Combine with other datasets (on-chain data, gas costs)

**Citation:**
```bibtex
@dataset{cpmm_results_2025,
  title={Statistical Results: Dual Vault vs Single Vault CPMM},
  author={CPMM Research Team},
  year={2025},
  note={12,000 Monte Carlo simulations, 3 concurrency levels},
  url={https://github.com/Gorocy/cpmm-rebase/tree/main/final_output}
}
```

## Related Documentation

- [../simulation/README.md](../simulation/README.md) - How data was generated
- [../analysis/README.md](../analysis/README.md) - How statistics were computed
- [../bigquery/README.md](../bigquery/README.md) - Parameter justification from on-chain data

## File Formats

### CSV Specification

- **Encoding:** UTF-8
- **Line endings:** LF (Unix-style)
- **Delimiter:** Comma (`,`)
- **Decimal separator:** Period (`.`)
- **No quoting** (data contains no commas)

**Compatibility:** Excel, Google Sheets, pandas, R, Stata, SPSS

### Notebook Format

- **Format:** Jupyter Notebook (.ipynb)
- **Kernel:** Python 3
- **Cell types:** Markdown (documentation) + Code (executable)

## Disclaimer

Statistical results reflect performance under simulated market conditions with specific assumptions (GBM price model, Bernoulli user activity, zero-threshold arbitrage). Real-world performance may differ due to:

- Network latency and failures
- More sophisticated arbitrageur strategies
- Correlated user behavior (herding, panic selling)
- External market integration (CEX prices, cross-chain arbitrage)
- Smart contract bugs or economic exploits

Always conduct independent testing before production deployment.

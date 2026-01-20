# On-Chain Data Analysis

Real-world blockchain data analysis using Google BigQuery to validate simulation parameters and justify experimental design choices. This directory contains SQL queries, empirical throughput measurements, and LaTeX documentation analyzing Solana DEX activity patterns.

## Purpose

The simulation parameters (particularly N=128 as "high concurrency" benchmark) are not arbitrary but derived from empirical analysis of actual Solana mainnet transactions. This analysis provides:

1. **Parameter justification:** Evidence-based selection of user count levels (32/128/512)
2. **Market context:** Understanding of real-world DEX throughput during high-activity periods
3. **Scalability bounds:** Identification of Per-Account Write Lock constraints
4. **Research validation:** Comparison of synthetic vs actual market conditions

## Data Source

All results were derived from the public dataset:
bigquery-public-data.crypto_solana_mainnet_us

The dataset contains publicly available blockchain data.
Published files contain only aggregated and derived statistics
and do not include raw transaction-level data.

This Google-maintained dataset contains complete on-chain transaction history including:
- Transaction signatures and metadata
- Account read/write operations  
- Compute unit consumption
- Block and slot information
- Program invocations

**Coverage:** January 2023 - Present (continuously updated)

**Update frequency:** Near real-time (< 1 hour lag)

**Data quality:** Validated by Google Cloud team, matches official Solana RPC data

## Analysis Period

**Window:** January 19, 2025 (04:00 UTC) - January 23, 2025 (00:00 UTC)

**Rationale:** This 4-day period captured extreme market volatility triggered by the "Trump Coin" memecoin launch on January 17, 2025. This event generated unprecedented DEX activity, providing ideal conditions for measuring peak throughput under stress.

**Characteristics:**
- High retail trading volume (FOMO-driven)
- Bot activity (wash trading, sniping, MEV)
- Network congestion events
- Representative of "viral token launch" scenarios

## Methodology

### Sliding Window Analysis

The analysis divides the time period into overlapping 2000-slot windows (≈13.3 minutes each):

1. **Window identification:** SQL query scans blockchain for consecutive 2000-slot sequences
2. **Transaction aggregation:** Count all swap transactions per window per trading pair
3. **Peak detection:** Identify windows with maximum transaction density
4. **Protocol filtering:** Group by DEX (Raydium CPMM, Meteora DLMM, Orca Whirlpool)

This methodology matches the simulation horizon (2000 slots) for direct comparability.

### Query: Maximum TPS per Trading Pair

See [max_tps.sql](max_tps.sql) for full implementation.

**Output:** [top_pairs_2000slots.csv](top_pairs_2000slots.csv)

## Key Findings

### Peak Throughput Measurements

Analysis of 409 high-activity windows revealed:

| DEX | Max TX/slot | Avg Top-4 TX/slot | Protocol Notes |
|-----|-------------|-------------------|----------------|
| Meteora DLMM | 143.6 | 106.3 | Bin-based liquidity, no explicit multi-pool sync |
| Raydium CPMM | 33.6 | 30.3 | Traditional CPMM, multiple fee tiers |
| Orca Whirlpool | 32.7 | 28.2 | Concentrated liquidity (Uniswap v3 style) |

**Interpretation caveats:**

1. **Survivor bias:** Only successful (confirmed) transactions are recorded. Rejected transactions in validator queues are invisible.

2. **Bot noise:** Significant portion of volume consists of wash trading and artificial liquidity manipulation, particularly during memecoin launches.

3. **Fee tier fragmentation:** Same trading pair exists across multiple pools with different fee structures (0.01%, 0.05%, 0.3%, 1%), fragmenting demand.

### Parameter Selection Justification

**N=32 (Low Concurrency):**
- Represents ~16 TX/slot (at p=0.5 activity rate)
- Typical for newly launched tokens or low-liquidity pairs
- Below empirical observations but useful for baseline comparison

**N=128 (High Concurrency):**
- Represents ~64 TX/slot
- **Conservative estimate** of realistic high-activity conditions
- Accounts for:
  - Fee tier fragmentation (demand split across multiple pools)
  - Rejected transactions (not visible in on-chain data)
  - Organic vs bot volume (real user demand likely lower than recorded volume)
- Matches observed averages for top pairs (~60 TX/slot)

**N=512 (Extreme Congestion):**
- Represents ~256 TX/slot
- Exceeds empirical observations (max ~144 TX/slot)
- **Stress test scenario** for:
  - DDoS attacks
  - Coordinated bot manipulation
  - Future growth projections
  - Per-Account Write Lock boundary testing

### Per-Account Write Lock Analysis

Solana enforces a 12M CU limit per account per slot. Combined with average swap cost:

- Single Vault: ~31k CU per swap → 387 TX/slot theoretical max
- Dual Vault: ~59k CU per swap → 203 TX/slot theoretical max

At N=512 with k=8 shards and probabilistic load distribution:
- Expected peak per vault: ~70 TX/slot (7σ analysis)
- Utilization: 35% of theoretical limit (safe margin)

This validates k=8 as sufficient fragmentation without excessive overhead.

## Files

### SQL Queries

[max_tps.sql](max_tps.sql) - Peak throughput analysis
- Identifies 2000-slot windows with maximum transaction density
- Groups by DEX protocol and trading pair
- Calculates per-slot averages and percentiles

### Data Outputs

[top_pairs_2000slots.csv](top_pairs_2000slots.csv) - Empirical measurements
- Top 50 trading pairs ranked by activity
- Peak and average TX/slot per window
- Protocol attribution (Raydium, Meteora, Orca)

## Running Queries

### Prerequisites

1. **Google Cloud account** with BigQuery access
2. **Project ID** with billing enabled
3. **BigQuery CLI** or web console access

### Execution

```bash
# Via bq CLI tool
bq query --use_legacy_sql=false < max_tps.sql

# Or via web console at https://console.cloud.google.com/bigquery
```

**Cost estimate:** ~$0.50 per query execution (depends on date range)

**Runtime:** 30-90 seconds for 4-day window

## Related Documentation

- [../simulation/README.md](../simulation/README.md) - Parameter configuration rationale
- [../analysis/README.md](../analysis/README.md) - Statistical processing pipeline
- [../final_output/README.md](../final_output/README.md) - Aggregated results

## Disclaimer

This analysis is for research purposes only. Throughput measurements should not be interpreted as performance guarantees for production systems or recommendations for parameter selection in live deployments.

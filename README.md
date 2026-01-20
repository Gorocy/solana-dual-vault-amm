# CPMM Rebalancing Research

Experimental research project implementing and evaluating a novel **Dual Vault architecture** for sharded Constant Product Market Makers (CPMM) on Solana blockchain. The project addresses the price desynchronization problem in fragmented liquidity pools through a passive synchronization mechanism based on vault pair rotation.

## Research Context

Traditional sharded AMM designs distribute liquidity across multiple independent pools to achieve horizontal scalability and overcome per-account write lock limitations in Solana. However, this fragmentation introduces severe price desynchronization between shards, creating arbitrage opportunities that extract value from liquidity providers (LVR - Loss Versus Rebalancing).

The **Dual Vault architecture** implements a round-robin rotation mechanism where *2N* vaults form *N* active pairs per slot. Each swap executes a "virtual swap" in the partner vault to maintain price consistency without requiring external arbitrage intervention. The **Single Vault** configuration (8 independent shards) serves as the baseline reference architecture.

This repository contains the complete research infrastructure: Solana smart contract, Monte Carlo simulation engine with market modeling, statistical analysis pipeline, on-chain data queries, and visualization notebooks.

## Repository Structure

```
cpmm-rebalansing/
├── programs/cpmm-rebalansing/   # Anchor smart contract (Solana program)
├── simulation/                   # Monte Carlo simulation engine (LiteSVM) - See simulation/README.md
├── analysis/                     # Statistical analysis tool (Rust + Polars) - See analysis/README.md
├── bigquery/                     # On-chain data analysis & LaTeX documentation - See bigquery/README.md
└── final_output/                # Aggregated statistics - See final_output/README.md
```

## Components

### 1. Smart Contract ([programs/cpmm-rebalansing/](programs/cpmm-rebalansing/))

Solana program written in Anchor framework implementing:

- **Single Vault** - Traditional sharded CPMM with independent fragments
- **Dual Vault** - Novel architecture with *2N* vaults forming *N* rotating pairs per slot

**Key Features:**
- Constant product formula (x * y = k) with fee option
- Horizontal scalability through vault fragmentation
- Virtual swap mechanism for passive synchronization (Dual Vault only)
- Round-robin pair rotation using Solana slot number

**Instructions:**
- `initialize_vault_registry` - Deploy vault registry with token pair configuration
- `initialize_vault` - Initialize individual vault
- `add_liquidity` - Deposit tokens, receive LP tokens
- `remove_liquidity` - Burn LP tokens, withdraw reserves
- `dual swap` - Execute token swap with virtual swap for reserves proportion synchronization
- `single swap` - Execute token swap without virtual swap ( only for simulation ) 

## 2. Simulation Engine ([simulation/README.md](simulation/README.md))

Monte Carlo simulation framework using [LiteSVM](https://github.com/LiteSVM/litesvm) (lightweight Solana Virtual Machine).

**Architecture:**
- Simulates 2000 slots (~13 minutes at 400ms/slot)
- Supports 3 concurrency levels: N ∈ {32, 128, 512} users
- Implements Geometric Brownian Motion (GBM) for external price evolution
- Includes 8 arbitrage bots monitoring price discrepancies

**Scenarios:**
- `single` - Single Vault with active arbitrage
- `single_no_arb` - Single Vault without arbitrage (isolated)
- `dual` - Dual Vault with active arbitrage
- `dual_no_arb` - Dual Vault without arbitrage (isolated)

See [simulation/README.md](simulation/README.md) for detailed configuration, execution instructions, and parameter documentation.

## Execution single round

*Install Anchor 'https://www.anchor-lang.com/docs/installation'*
*Build Program*
```bash
anchor build
```


```bash
cargo run --release --bin sim_dual
```

## Example execution Monte Carlo 
```bash
cargo run --release --bin dual_32
```
### Check ([simulation/src/bin](simulation/src/bin))*

**Output Format:**
- `seed_N.csv` - Vault reserves, user balances, arbitrageur state per slot
- `seed_N_compute.csv` - Compute unit consumption per transaction

## 3. Analysis Tool ([analysis/README.md](analysis/README.md))

Parallel batch processor for aggregating Monte Carlo results. Uses Rust with Polars (DataFrame library) and Rayon (parallelism).

**Processing Pipeline:**
1. Discovers 12 scenarios (3 user counts × 2 architectures × 2 market conditions)
2. Loads ~1000 simulation runs per scenario
3. Computes per-run metrics (spread, RMSE, LP profit, IL, arbitrage profit)
4. Aggregates statistics (mean, standard deviation, 95th percentile)
5. Generates histograms (100 bins) for distribution analysis

**Metrics:**
- Price stability: average spread, max spread, RMSE
- LP economics: profit value, profit per 1M volume, impermanent loss
- Arbitrage: total profit, trade count, intervention rate
- Computational efficiency: total CU, average CU per swap

**Usage:**
```bash
cargo run -p analysis --release
```

**Output:**
- `final_summary.csv` - 12 rows (scenarios) × 41 columns (metrics with mean/stddev/p95)
- `histograms.csv` - Distribution data for visualization

See [analysis/README.md](analysis/README.md) for detailed documentation.

## 4. On-Chain Data Analysis ([bigquery/README.md](bigquery/README.md))

Google BigQuery SQL queries and documentation analyzing real-world Solana DEX activity.

**Data Source:** `bigquery-public-data.crypto_solana_mainnet_us`

**Analysis:**
- Top trading pairs by activity (Raydium CPMM, Meteora DLMM, Orca Whirlpool)
- Peak transaction rates (TX/slot) during congestion events
- Justification for N=128 as "high concurrency" benchmark

**Files:**
- [max_tps.sql](bigquery/max_tps.sql) - Query for identifying peak throughput windows
- [top_pairs_2000slots.csv](bigquery/top_pairs_2000slots.csv) - Empirical throughput data
- [roz_6.tex](bigquery/roz_6.tex) - Research chapter with complete analysis (LaTeX)

See [bigquery/README.md](bigquery/README.md) for data provenance information.

## Experimental Design

### Monte Carlo Methodology

**Replications:** 1000 independent runs per scenario (seed range: 100-1099 or 1000-1999)

**Horizon:** 2000 Solana slots ≈ 13.3 minutes (400ms average slot time)

**External Price Model:**
- Geometric Brownian Motion (GBM)
- Volatility: σ = 0.8
- Drift: μ ∈ [-0.15, 0.15] (seed-dependent, simulates bull/bear/neutral markets)
- Price delta: 7-21% over simulation window

**User Behavior:**
- Type: Taker (market orders, no slippage limit)
- Activity: Bernoulli(p=0.5) per slot per user
- Direction bias: Correlated with price drift (momentum trading)

**Arbitrage Mechanism:**
- 8 agents (one per shard/pair)
- Threshold: 0% (react to any price deviation)
- Strategy: Execute swap to align vault price with external fair price

**Fragmentation Parameter:** k=8 shards (justified by Per-Account Write Lock analysis)

### Scenarios

| Scenario ID | Architecture | Users | Arbitrage | Avg TX/slot | Purpose |
|------------|--------------|-------|-----------|-------------|---------|
| `single_n32` | Single Vault | 32 | Yes | 16 | Low concurrency baseline |
| `single_no_arb_n32` | Single Vault | 32 | No | 16 | Isolated system behavior |
| `dual_n32` | Dual Vault | 32 | Yes | 16 | Low concurrency test |
| `dual_no_arb_n32` | Dual Vault | 32 | No | 16 | Synchronization effectiveness |
| `single_n128` | Single Vault | 128 | Yes | 64 | High concurrency reference |
| `single_no_arb_n128` | Single Vault | 128 | No | 64 | Realistic production load |
| `dual_n128` | Dual Vault | 128 | Yes | 64 | Target deployment scenario |
| `dual_no_arb_n128` | Dual Vault | 128 | No | 64 | Internal efficiency |
| `single_n512` | Single Vault | 512 | Yes | 256 | Extreme congestion stress test |
| `single_no_arb_n512` | Single Vault | 512 | No | 256 | Network saturation |
| `dual_n512` | Dual Vault | 512 | Yes | 256 | Scalability limit |
| `dual_no_arb_n512` | Dual Vault | 512 | No | 256 | Maximum throughput |

## Key Findings

Results from 12,000 simulation runs (1000 per scenario):

### Price Stability
- **Spread reduction:** Dual Vault achieves 54-79% lower average spread vs Single Vault (no arbitrage conditions)
- **RMSE improvement:** 23-68% better price tracking of external market price
- Single Vault: 35-38% average spread (near-total desynchronization)
- Dual Vault: 8-16% average spread (maintained even at extreme load)

### LP Economics
- **Profit increase:** LP earnings +89% to +131% higher in Dual Vault (no arbitrage scenarios)
- **Arbitrage reduction:** External arbitrageur profits reduced by ~56% across all load levels
- **Value capture:** Dual Vault internalizes arbitrage value through virtual swap mechanism

### User Welfare
- **Cost savings:** 0.3-2.9% better execution prices for traders (with arbitrage scenarios)
- Improvement scales with network congestion (higher at N=512)

### Computational Cost
- **CU overhead:** Dual Vault requires +91% more Compute Units (~59k vs ~31k per swap)
- Remains within safe limits (200k CU transaction limit in Solana)
- Cost-benefit: Higher CU expenditure justified by LP profit increase and price stability

### Trade-offs
- **Higher Impermanent Loss:** Dual Vault experiences greater IL due to higher transaction volume (narrower spreads encourage more trades)
- **Net positive for LPs:** Despite higher IL, total LP returns are significantly better due to internalized arbitrage value

See [final_output/final_summary.csv](final_output/final_summary.csv) for complete statistical results. See [final_output/README.md](final_output/README.md) for detailed explanation of output files and visualization notebooks.

## Performance Benchmarks

**Simulation throughput:**
- Compute unit tracking: ~5000-62000 CU per transaction
- Memory footprint: ~100-300 MB per simulation process

**Analysis throughput:**
- Processing rate: ~200-400 CSV files/second (Polars + Rayon)
- Total data volume: ~150 GB (12 scenarios × ~1000 files × 2 MB avg)
- Memory requirement: 4-8 GB RAM

## Research Applications

This infrastructure supports the following research questions:

1. **Scalability-Stability Trade-off:** Can sharded AMMs achieve high throughput without sacrificing price consistency?
2. **LVR Mitigation:** Does passive synchronization reduce liquidity provider losses compared to active arbitrage?
3. **Cost-Benefit Analysis:** Is the computational overhead of virtual swaps justified by economic improvements?
4. **Congestion Resilience:** How do different architectures perform under network stress (DoS, wash trading)?

## Related Work

- [Sharded Automated Market Maker (SAMM)](https://arxiv.org/abs/2410.05253) - Chen et al., 2024
- [Automated Market Making and Loss-Versus-Rebalancing](https://arxiv.org/abs/2208.06046) - Milionis et al., 2022
- [Solana Outage Reports](https://solana.com/news/04-30-22-solana-mainnet-beta-outage-report-mitigation) - Network congestion case studies

## License

MIT License

## Disclaimer

This is experimental research software. The smart contracts have not been audited and should not be deployed to mainnet without thorough security review. Simulation results are based on synthetic market conditions and may not reflect real-world performance.

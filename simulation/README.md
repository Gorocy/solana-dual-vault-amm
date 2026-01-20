# Simulation Engine

Monte Carlo simulation framework for testing CPMM architectures (Single Vault vs Dual Vault) under various market conditions. Built using [LiteSVM](https://github.com/LiteSVM/litesvm) - a lightweight Solana Virtual Machine implementation that enables high-throughput deterministic testing without network overhead.

## Architecture Overview

The simulation engine models a complete decentralized exchange environment including:

- **Market participants:** N users (traders) with random balances and trading behavior
- **Liquidity pools:** 8 or 16 vaults depending on architecture (Single vs Dual)
- **Arbitrageurs:** 8 profit-seeking agents that monitor and exploit price discrepancies
- **External market:** Geometric Brownian Motion (GBM) price oracle simulating realistic volatility

Each simulation executes **2000 Solana slots** (approximately 13.3 minutes at 400ms per slot), capturing sufficient data for statistical analysis while maintaining computational feasibility for Monte Carlo studies.

## Key Components

### Market Price Model (GBM)

The external "fair price" evolves according to Geometric Brownian Motion:

```
dS/S = μdt + σdW
```

Where:
- **μ (drift):** Annual expected return in range [-0.15, 0.15], seed-dependent
  - Positive drift: Bull market (users favor A→B swaps)
  - Negative drift: Bear market (users favor B→A swaps)
  - Near-zero drift: Neutral market (50/50 swap direction)
- **σ (volatility):** 0.8 annual volatility (80%), calibrated to crypto market conditions
- **dW:** Wiener process (Brownian motion incremental)

Time steps are scaled to Solana slots: Δt ≈ 0.4 seconds. Over 2000 slots, typical price movement ranges 7-21%.

### User Behavior Model

Each user independently decides whether to trade each slot with:

- **Activity probability:** p = 0.5 (Bernoulli trial)
- **Swap direction:** Biased toward market drift
  - Bull market: P(A→B) ≈ 0.6
  - Bear market: P(A→B) ≈ 0.4
  - Neutral: P(A→B) = 0.5
- **Swap size:** Uniform random in [10%, 30%] of current balance
- **Order type:** Market order (no slippage protection, min_amount_out = 1)

This models retail "momentum traders" who follow trends rather than countertrading.

### Arbitrage Model

Arbitrageurs continuously monitor price differences between:
- Vault prices vs external market price (GBM oracle)
- Inter-vault price discrepancies (Single Vault architecture only)

**Activation threshold:** 0% - arbitrageurs react to any price deviation, representing the most aggressive (and worst-case for LPs) arbitrage scenario.

**Strategy:** Execute swaps to align vault price with external market price, capturing the spread as profit.

**Count:** 8 agents (one per shard/vault pair) to ensure full coverage.

## Configuration Parameters

See [src/bin/SCENARIOS.MD](src/bin/SCENARIOS.MD) for comprehensive parameter documentation.

### Core Constants

| Parameter | Value | Description |
|-----------|-------|-------------|
| `MAX_SLOTS_SIMULATION` | 2000 | Simulation duration in Solana slots |
| `VAULTS_FOR_SIMULATION` | 8 | Number of active vault pairs (16 total vaults for Dual) |
| `USERS_FOR_SIMULATION` | 32/128/512 | Concurrency level (low/high/extreme) |
| `INITIAL_LIQUIDITY_A_VAULT` | 100k | Token A reserves per vault (Dual) |
| `INITIAL_LIQUIDITY_B_VAULT` | 200k | Token B reserves per vault (Dual) |
| `FEE_OPTION_BASE` | 30 | Protocol fee in basis points (0.3%) |
| `MARKET_VOLATILITY` | 0.8 | Annual volatility for GBM model |
| `SWAP_FREQUENCY` | 0.5 | User activity probability per slot |
| `ARBITRAGEUR_COUNT` | 8 | Number of arbitrage bots |
| `ARBITRAGE_FREQUENCY` | 0.9 | Arbitrageur monitoring probability |
| `MIN_ARBITRAGE_PROFIT` | 0.0 | Minimum profit threshold (0 = aggressive) |

**Note:** Single Vault configurations use 2x larger reserves per vault (200k/400k) to match total liquidity with Dual Vault (16 vaults × 100k/200k).

## Scenarios

Four experimental conditions per concurrency level (12 total scenarios):

### 1. Single Vault with Arbitrage (`single`)

**Purpose:** Baseline reference architecture

Traditional fragmented AMM where 8 independent vaults operate without coordination. External arbitrageurs maintain price consistency by exploiting inter-vault differences.

**Expected behavior:**
- Prices converge due to arbitrage activity
- High arbitrage profits (LVR extraction from LPs)
- Lower spreads than isolated scenario

### 2. Single Vault without Arbitrage (`single_no_arb`)

**Purpose:** Isolated system behavior analysis

Same architecture but arbitrageurs disabled to observe "natural" price divergence.

**Expected behavior:**
- Severe price desynchronization (>30% spread)
- No arbitrage profits
- Demonstrates fragmentation problem severity

### 3. Dual Vault with Arbitrage (`dual`)

**Purpose:** Target production scenario

Novel architecture with 16 vaults forming 8 rotating pairs. Virtual swaps provide passive synchronization. External arbitrageurs still active.

**Expected behavior:**
- Prices remain synchronized via internal mechanism
- Reduced arbitrage opportunities
- LPs capture value that would otherwise leak to arbitrageurs

### 4. Dual Vault without Arbitrage (`dual_no_arb`)

**Purpose:** Synchronization effectiveness proof

Tests whether internal mechanism alone (without external arbitrageurs) maintains price consistency.

**Expected behavior:**
- Moderate spreads (<15%) despite no arbitrage
- Demonstrates self-stabilizing property
- Proves round-robin rotation effectiveness

## Concurrency Levels

Three user count configurations test scalability:

### N=32 (Low Concurrency)

- **Expected TX/slot:** ~16 (32 × 0.5 activity rate)
- **Scenario:** Normal market conditions, token launch phase
- **Per-Account Load:** ~2 TX/slot per vault (well below limits)

### N=128 (High Concurrency)

- **Expected TX/slot:** ~64
- **Scenario:** High-activity market, popular trading pairs
- **Justification:** Empirically derived from on-chain data (see [../bigquery/](../bigquery/))
- **Per-Account Load:** ~8 TX/slot per vault (conservative estimate)

### N=512 (Extreme Congestion)

- **Expected TX/slot:** ~256
- **Scenario:** Stress test for network saturation (DDoS, wash trading)
- **Per-Account Load:** ~32 TX/slot per vault (approaching theoretical limits)
- **Statistical buffer:** 7σ analysis shows peak load ~70 TX/slot (35% of 200 TX limit)

## Execution

### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Anchor framework
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest

# Build smart contract
anchor build
```

### Single Simulation Run

Execute one simulation (useful for debugging):

```bash
cd simulation

# Dual Vault, 128 users, seed 100
cargo run --release --bin sim_dual -- --users 128 --seed 100

# Single Vault, 128 users, seed 100
cargo run --release --bin sim_single -- --users 128 --seed 100
```

**Output files:**
- `seed_100.csv` - Vault state evolution (reserves, prices, balances)
- `seed_100_compute.csv` - Transaction-level compute units and success flags

### Monte Carlo Batch Execution

Run complete Monte Carlo study for one scenario (1000 replications):

```bash
# Execute binary for specific scenario
cargo run --release --bin dual_32        # Dual Vault, N=32, seeds 1000-1999
cargo run --release --bin dual_no_arb_32 # Dual Vault no arb, N=32
cargo run --release --bin single_128     # Single Vault, N=128
# ... etc. (see src/bin/ for all 12 binaries)
```

**Available binaries:**

| Binary | Architecture | Users | Arbitrage | Output Directory |
|--------|--------------|-------|-----------|------------------|
| `dual_32` | Dual Vault | 32 | Yes | `../32/dual/` |
| `dual_no_arb_32` | Dual Vault | 32 | No | `../32/dual_no_arb/` |
| `single_32` | Single Vault | 32 | Yes | `../32/single/` |
| `single_no_arb_32` | Single Vault | 32 | No | `../32/single_no_arb/` |
| `dual_128` | Dual Vault | 128 | Yes | `../128/dual/` |
| `dual_no_arb_128` | Dual Vault | 128 | No | `../128/dual_no_arb/` |
| `single_128` | Single Vault | 128 | Yes | `../128/single/` |
| `single_no_arb_128` | Single Vault | 128 | No | `../128/single_no_arb/` |
| `dual_512` | Dual Vault | 512 | Yes | `../512/dual/` |
| `dual_no_arb_512` | Dual Vault | 512 | No | `../512/dual_no_arb/` |
| `single_512` | Single Vault | 512 | Yes | `../512/single/` |
| `single_no_arb_512` | Single Vault | 512 | No | `../512/single_no_arb/` |

Each binary executes 1000 simulations sequentially with seeds 1000-1999.

**Execution time:** ~15-45 minutes per binary (depending on CPU, scenario complexity)

**Total data generation:** ~12-24 hours for all 12 scenarios (parallelizable across multiple machines)

## Output Format

### Reserve State CSV (`seed_N.csv`)

Captures complete system state at each slot:

```csv
slot,market_price,vault_0_price,vault_0_reserve_a,vault_0_reserve_b,...,arb_0_balance_a,arb_0_balance_b,user_0_balance_a,user_0_balance_b,...
0,1.000000,1.000000,100000000,200000000,...,5000000000,10000000000,5000000,10000000,...
1,1.001234,1.000156,99950000,200100000,...,5000050000,9999950000,4995000,10005000,...
...
```

**Columns:**
- `slot` - Time index (0-1999)
- `market_price` - External GBM price (oracle)
- `vault_N_price` - Spot price in vault N: reserve_b / reserve_a
- `vault_N_reserve_a` - Token A balance in vault N (lamports)
- `vault_N_reserve_b` - Token B balance in vault N (lamports)
- `arb_N_balance_a/b` - Arbitrageur N holdings (if enabled)
- `user_N_balance_a/b` - User N holdings (if feature `track_user_balances` enabled)

**File size:** ~2-5 MB per simulation (depends on vault count and feature flags)

### Compute Units CSV (`seed_N_compute.csv`)

Transaction-level computational cost tracking:

```csv
slot,compute_units,success,amount_in
0,58234,true,1000000
0,59102,true,1500000
1,31456,true,500000
1,0,false,2000000
...
```

**Columns:**
- `slot` - Slot when transaction executed
- `compute_units` - CU consumed (0 if failed)
- `success` - Transaction success flag
- `amount_in` - Input token amount (lamports)

**Purpose:** Enables analysis of computational efficiency (CU per dollar volume) and failure rates.

## Code Structure

```
simulation/
├── Cargo.toml           # Dependencies and feature flags
├── src/
│   ├── lib.rs           # Core simulation logic
│   ├── error.rs         # Custom error types
│   ├── output.rs        # CSV serialization
│   ├── utils/           # Helper functions
│   │   ├── gbm.rs       # Geometric Brownian Motion
│   │   ├── math.rs      # CPMM calculations
│   │   └── vault.rs     # Vault state management
│   ├── scenarios/       # Scenario configurations
│   │   ├── dual.rs      # Dual Vault setup
│   │   └── single.rs    # Single Vault setup
│   └── bin/             # Executable binaries (12 scenarios)
│       ├── dual_32.rs
│       ├── dual_128.rs
│       ├── dual_512.rs
│       ├── ...
│       └── SCENARIOS.MD # Parameter documentation
└── tests/               # Unit tests
```

## Feature Flags

Configure compilation via `Cargo.toml` features:

- `track_user_balances` - Record per-user balance evolution (adds ~30% overhead)
- `min-output` - Minimal slippage protection (default: enabled)
- `payer-sign` - Transaction signing simulation (default: enabled)
- `create-user-lp-token-account` - LP token account creation (default: enabled)

Disable expensive features for faster batch execution:

```bash
cargo run --release --no-default-features --features min-output --bin dual_128
```

## Performance Optimization

**Parallelization:** Each scenario binary is independent, enabling cluster parallelization:

```bash
# Example: GNU Parallel on 12-core machine
seq 1000 1999 | parallel -j 12 'cargo run --release --bin dual_128 -- --seed {}'
```

**Memory management:** LiteSVM maintains ~100-300 MB per simulation. Safe to run 4-8 concurrent simulations per GB of RAM.

**Disk I/O:** Use SSD storage for output directories. HDD bottleneck can reduce throughput by 3-5x.

## Validation and Testing

```bash
# Run unit tests
cargo test

# Run integration test (single round)
cargo run --release --bin sim_dual -- --users 32 --seed 42

# Verify output format
head -n 5 seed_42.csv
wc -l seed_42.csv  # Should show 2001 lines (header + 2000 slots)
```

## Troubleshooting

**Error: "Program account does not exist"**
- Run `anchor build` to compile smart contract before simulation

**Error: "Insufficient compute units"**
- Check that CU limit (1_400_000) is sufficient for transaction size
- Large swaps may exceed limits in extreme congestion scenarios

**Slow execution (>1 min per simulation)**
- Disable `track_user_balances` feature
- Verify release mode compilation (`--release` flag)
- Check CPU thermal throttling

**CSV parse errors in downstream analysis**
- Ensure no interruptions during execution (crashes leave partial files)
- Verify disk space availability (each scenario generates ~2-5 GB)

## Research Extensions

This simulation framework can be adapted for:

1. **Alternative fee structures:** Modify `FEE_OPTION_BASE` to test dynamic fees
2. **Different vault counts:** Change `VAULTS_FOR_SIMULATION` to study fragmentation scaling
3. **Custom market models:** Replace GBM with alternative stochastic processes
4. **Smart order routing:** Implement aggregator logic for multi-vault execution
5. **MEV strategies:** Add sophisticated arbitrage bots with sandwich attacks

## Related Documentation

- [../analysis/README.md](../analysis/README.md) - Statistical analysis pipeline
- [../programs/cpmm-rebalansing/](../programs/cpmm-rebalansing/) - Smart contract source
- [src/bin/SCENARIOS.MD](src/bin/SCENARIOS.MD) - Detailed parameter documentation

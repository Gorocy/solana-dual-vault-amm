# Comprehensive Simulation Configuration

## 1. Global Simulation Constants

| Constant | Value | Description |
| --- | --- | --- |
| `MAX_SLOTS_SIMULATION` | `2,000` | The duration of the test run measured in Solana slots (rounds). At 0.4s per slot, this equals ~6-7 minutes of simulated time. |
| `VAULTS_FOR_SIMULATION` | `8` | Total number of vaults. Dual swap uses 2x more vaults (16) than single swap (8) for equivalent throughput. |

## 2. User & Economic Parameters

| Constant | Value | Description |
| --- | --- | --- |
| `USERS_FOR_SIMULATION` | `128` | The baseline number of independent trading agents. |
| `BALANCE_MIN_A_USER` | `1k` | Minimum starting balance of Asset A for a simulated user. |
| `BALANCE_MAX_A_USER` | `10k` | Maximum starting balance of Asset A for a simulated user. |
| `BALANCE_MIN_B_USER` | `2k` | Minimum starting balance of Asset B for a simulated user. |
| `BALANCE_MAX_B_USER` | `20k` | Maximum starting balance of Asset B for a simulated user. |

## 3. Vault & Market Logic

| Constant | Value | Description |
| --- | --- | --- |
| `INITIAL_LIQUIDITY_A_VAULT` | `100k` | Starting reserve of Asset A per vault. Single swap has 2x larger reserves per vault (200k) to match total liquidity. |
| `INITIAL_LIQUIDITY_B_VAULT` | `200k` | Starting reserve of Asset B per vault. Single swap has 2x larger reserves per vault (400k) to match total liquidity. |
| `SWAP_FREQUENCY` | `0.05` | 5% probability that a user will attempt a swap in any given slot. |
| `MIN_SWAP_PERCENTAGE` | `0.10` | Minimum swap size (10% of user's current balance). |
| `MAX_SWAP_PERCENTAGE` | `0.30` | Maximum swap size (30% of user's current balance). |
| `FEE_OPTION_BASE` | `30` | The base protocol fee set at 30 basis points (0.3%). |
| `MARKET_VOLATILITY` | `0.8` | Annual volatility (80%) for the GBM market price model for crypto markets. |
| `MARKET_DRIFT` | `[-0.15, +0.15]` | Annual drift calculated from simulation seed. Positive = bullish trend, negative = bearish. User behavior (A→B probability) is correlated with market drift. |

**Market Price Simulation:** Uses Geometric Brownian Motion.

## 4. Arbitrage Parameters

| Constant | Value | Description |
| --- | --- | --- |
| `ENABLE_ARBITRAGEURS` | `true` | Boolean flag to toggle the presence of external profit-seeking agents. |
| `ARBITRAGEUR_COUNT` | `8` | Number of arbitrage agents (derived from `VAULTS_FOR_SIMULATION`). |
| `ARBITRAGE_FREQUENCY` | `0.9` | 90% probability of an arbitrageur checking for price gaps per slot. |
| `MIN_ARBITRAGE_PROFIT` | `0.0` | Minimum profit threshold required for an arbitrageur to execute a trade. |

---

# Experimental Methodology: Comparative Scenarios

To validate the **Multi-Vault (Dual Vault)** hypothesis, the simulation iterates through the following four scenarios using the constants above as the baseline.

### Scenario 1: Classic AMM + User Noise

* **Conditions:** `ENABLE_ARBITRAGEURS: false`, Standard  logic.
* **Expected Result:** High price divergence between the 8 vaults. This illustrates the inefficiency of fragmented liquidity when only "noisy" retail flow is present.

### Scenario 2: Classic AMM + Arbitrage (Benchmark)

* **Conditions:** `ENABLE_ARBITRAGEURS: true`, Standard  logic.
* **Expected Result:** Prices remain aligned across vaults, but at the cost of **Arbitrage Leakage**.

### Scenario 3: Dual Vault + User Noise

* **Conditions:** `ENABLE_ARBITRAGEURS: false`, Multi-Vault internal pairing enabled.
* **Expected Result:** **The Core Hypothesis.** Prices should remain significantly more stable across vaults than in Scenario 1, proving that the internal pairing mechanism can mitigate divergence without needing external arbitrage.

### Scenario 4: Dual Vault + Arbitrage (Optimized Market)

* **Conditions:** `ENABLE_ARBITRAGEURS: true`, Multi-Vault internal pairing enabled.
* **Expected Result:** Price stability with lower total profits captured by arbitrageurs compared to Scenario 2, indicating better value retention for Liquidity Providers.

---


# Price Rebalancing: Scaling Analysis

The core of this analysis is to verify the efficiency of the **Dual Swap** mechanism - inherent rebalancing logic. By testing with different values of `USERS_FOR_SIMULATION` (32, 128, 512), we examine if price remains consistent when the density of swap instructions fluctuates. Note that `USER_SWAP_FREQUENCY` is set to 50% - intentionally high to create significant market noise and stress-test the rebalancing mechanism under high trading volume.

## 1. Low-Density Market (n=32)

This scenario investigates the model's performance with minimal user count. With 32 users and 50% swap frequency, we expect ~16 swaps per slot. Since rebalancing is embedded within each swap, this tests whether the internal logic maintains price convergence with limited number of participants but high activity per user. We monitor if the small user base allows prices to drift between vaults despite frequent trading.

## 2. Baseline Density (n=128)

This is the standard configuration with 128 users and 50% swap frequency, generating ~64 swaps per slot. This represents typical market conditions with moderate participant count and serves as the reference point for comparing low-density and high-density scenarios. We evaluate price stability and arbitrage efficiency at this baseline level under significant market noise.

## 3. High-Density Market (n=512)

This scenario simulates conditions of intense market activity with 512 users generating ~256 swaps per slot. The objective is to measure the robustness of the internal math under extreme throughput: can the atomic rebalancing within each swap effectively maintain price consistency when processing very high volume of retail orders? We monitor for mathematical stability and cumulative price drift when vault states are updated in rapid succession with intentionally high noise levels.

*Important: In each slot, the vaults rotate the pair used for dual swap rebalancing, ensuring all vaults remain synchronized over time.*

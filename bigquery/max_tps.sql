-- Define analysis time range
DECLARE start_ts TIMESTAMP DEFAULT TIMESTAMP('2025-01-19 04:00:00 UTC');
DECLARE end_ts   TIMESTAMP DEFAULT TIMESTAMP('2025-01-23 00:00:00 UTC');

WITH known_dexes AS (
  -- ============================
  -- Known DEX program IDs (allowlist)
  -- ============================

  -- RAYDIUM (V4, CLMM, CPMM)
  SELECT '675kPX9MMc7RSHm2bsWXEVcaTkSoxGSfGfRkpK6Y86X1' AS program_id, 'Raydium V4 (Legacy)' AS dex_name UNION ALL
  SELECT 'CAMMCzo5YL8w4VFF8KVHrSgSvnAByfE5p4EzWSCre6V', 'Raydium CLMM' UNION ALL
  SELECT 'CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C', 'Raydium CPMM' UNION ALL

  -- ORCA (Whirlpool)
  SELECT 'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc', 'Orca Whirlpool' UNION ALL

  -- METEORA (DLMM + Dynamic Pools)
  SELECT 'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo', 'Meteora DLMM' UNION ALL
  SELECT 'Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB', 'Meteora Dynamic'
),

raw_data AS (
  -- ============================
  -- Base transaction-level extraction
  -- ============================
  -- Each row represents a writable account touched by a transaction
  -- that interacted with at least one known DEX program

  SELECT
    t.block_slot,
    DIV(t.block_slot, 2000) AS window_id,        -- Slot window (~2000 slots)
    acc.pubkey AS pool_address,                  -- Candidate pool account
    t.err,                                       -- Transaction error (empty = success)

    -- Extract token mints owned by this pool in pre-state
    ARRAY_TO_STRING(
      ARRAY(
        SELECT DISTINCT mint
        FROM UNNEST(t.pre_token_balances) bal
        WHERE bal.owner = acc.pubkey
        ORDER BY mint
      ),
      ','
    ) AS token_mints,

    -- Identify which known DEX program was involved in the transaction
    (
      SELECT ANY_VALUE(k.dex_name)
      FROM UNNEST(t.accounts) a
      JOIN known_dexes k
        ON a.pubkey = k.program_id
    ) AS dex_name

  FROM
    `bigquery-public-data.crypto_solana_mainnet_us.Transactions` t,
    UNNEST(t.accounts) acc

  WHERE
    -- Time filter
    t.block_timestamp >= start_ts
    AND t.block_timestamp < end_ts

    -- Only writable accounts (state-changing)
    AND acc.writable = true

    -- Exclude system / non-pool accounts
    AND acc.pubkey NOT IN (
      'ComputeBudget111111111111111111111111111111',
      '11111111111111111111111111111111',
      'SysvarC1ock11111111111111111111111111111111',
      'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
    )

    -- Keep only transactions that interacted with a known DEX program
    AND EXISTS (
      SELECT 1
      FROM UNNEST(t.accounts) a
      JOIN known_dexes k
        ON a.pubkey = k.program_id
    )
),

slot_stats AS (
  -- ============================
  -- Step 1: Aggregate per SLOT
  -- ============================
  -- Measures congestion at slot granularity per pool

  SELECT
    window_id,
    block_slot,
    pool_address,

    ANY_VALUE(token_mints) AS token_mints,
    ANY_VALUE(dex_name)   AS dex_name,

    COUNT(*) AS tx_count_in_slot,                 -- Total tx touching pool in this slot
    COUNTIF(err = '') AS failed_tx_in_slot        -- Failed tx count (err populated)

  FROM raw_data
  WHERE
    dex_name IS NOT NULL        -- Safety: only allowlisted DEXes
    AND token_mints != ''       -- Ensure pool actually holds tokens

  GROUP BY
    window_id,
    block_slot,
    pool_address
),

window_pool_stats AS (
  -- ============================
  -- Step 2: Aggregate per WINDOW and per POOL
  -- ============================
  -- Produces congestion statistics for each pool over a ~2000-slot window

  SELECT
    window_id,
    pool_address,

    ANY_VALUE(token_mints) AS token_mints,
    ANY_VALUE(dex_name)   AS dex_name,

    SUM(tx_count_in_slot)     AS total_tx_in_window,
    SUM(failed_tx_in_slot)   AS total_failed_in_window,

    COUNT(block_slot)        AS slots_active_count,  -- Number of slots where pool was touched

    -- High-percentile congestion metrics (burstiness)
    APPROX_QUANTILES(tx_count_in_slot, 1000)[OFFSET(950)] AS p95_tx_per_slot,
    APPROX_QUANTILES(tx_count_in_slot, 1000)[OFFSET(990)] AS p99_tx_per_slot,
    APPROX_QUANTILES(tx_count_in_slot, 1000)[OFFSET(999)] AS p99_9_tx_per_slot,

    MIN(block_slot) AS start_slot,
    MAX(block_slot) AS end_slot

  FROM slot_stats
  GROUP BY
    window_id,
    pool_address
)

-- ============================
-- Step 3: Final aggregation per WINDOW
-- ============================
-- Returns top congested pools (JSON-style array) per window

SELECT
  window_id,

  MIN(start_slot) AS window_start_slot,
  MAX(end_slot)   AS window_end_slot,

  ARRAY_AGG(
    STRUCT(
      dex_name,
      pool_address,
      token_mints,
      total_tx_in_window,
      total_failed_in_window,
      slots_active_count,
      p95_tx_per_slot,
      p99_tx_per_slot,
      p99_9_tx_per_slot
    )
    ORDER BY total_tx_in_window DESC
    LIMIT 20
  ) AS top_congested_pools

FROM window_pool_stats
GROUP BY window_id
ORDER BY window_id;

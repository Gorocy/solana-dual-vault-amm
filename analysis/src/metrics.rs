use anyhow::Result;
use crate::LAMPORTS_PER_SOL_F64;
use crate::types::{ReservesRecord, ComputeRecord};
use crate::aggregation::RunMetrics;

/// Main function calculating all metrics for a single simulation run
pub fn calculate_run_metrics(
    reserves: &[ReservesRecord],
    compute: &[ComputeRecord],
    seed: u64,
) -> Result<RunMetrics> {
    if reserves.is_empty() {
        anyhow::bail!("Empty reserves data for seed {}", seed);
    }
    
    let first = &reserves[0];
    let last = reserves.last().unwrap();
    
    // ===== 1. WEALTH DISTRIBUTION  =====  
    // User PnL: Change in user portfolio value (priced in Token B)
    let user_pnl = calculate_user_pnl(first, last);
    
    // Arb Profit: Total profit of arbitrageurs
    let arb_profit = calculate_arb_profit(first, last);
    
    // LP Profit Value: LP nominal profit
    let lp_profit_value = calculate_lp_profit_value(first, last);
    
    // ===== 2. VOLUME & EFFICIENCY =====
    // Total Volume: Sum of amount_out from successful transactions
    let successful_swaps: Vec<_> = compute.iter().filter(|r| r.success == 1).collect();
    let total_volume: f64 = successful_swaps.iter()
        .map(|r| r.amount_out as f64 / LAMPORTS_PER_SOL_F64)
        .sum();
    
    // LP Profit per 1M Volume
    let lp_profit_per_1m_vol = if total_volume > 0.0 {
        (lp_profit_value / total_volume) * 1_000_000.0
    } else {
        0.0
    };
    
    // Arb Trade Count: Number of arbitrage trades (balance changes)
    let arb_trade_count = count_arb_trades(reserves);
    
    // Intervention Rate: Ratio of arb trades to all trades
    let total_swaps = compute.len() as u64;
    let intervention_rate = if total_swaps > 0 {
        arb_trade_count as f64 / total_swaps as f64
    } else {
        0.0
    };
    
    // ===== 3. STANDARD MARKET METRICS =====
    // Spread metrics (in %)
    let (avg_spread_pct, max_spread_pct) = calculate_spreads(reserves);
    
    // RMSE price from market
    let rmse_price = calculate_rmse_price(reserves);
    
    // Impermanent Loss (%)
    let impermanent_loss_pct = calculate_impermanent_loss(first, last);
    
    // ===== 4. COMPUTE METRICS =====
    let total_compute_units: u64 = compute.iter().map(|r| r.compute_units).sum();
    let avg_cu_per_swap = if total_swaps > 0 {
        total_compute_units as f64 / total_swaps as f64
    } else {
        0.0
    };
    
    Ok(RunMetrics {
        seed,
        // Wealth Distribution
        user_pnl,
        arb_profit,
        lp_profit_value,
        // Volume & Efficiency
        total_volume,
        lp_profit_per_1m_vol,
        arb_trade_count,
        intervention_rate,
        // Market Metrics
        avg_spread_pct,
        max_spread_pct,
        rmse_price,
        impermanent_loss_pct,
        // Compute
        total_compute_units,
        avg_cu_per_swap,
        total_swaps,
        successful_swaps: successful_swaps.len() as u64,
    })
}

/// Calculate change in user portfolio value (User PnL)
/// Priced in Token B at market price
fn calculate_user_pnl(first: &ReservesRecord, last: &ReservesRecord) -> f64 {
    match (first.user_balance_a, first.user_balance_b, last.user_balance_a, last.user_balance_b) {
        (Some(initial_a), Some(initial_b), Some(final_a), Some(final_b)) => {
            // Initial value: Token B + Token A * market price
            let initial_value = (initial_b as f64 + initial_a as f64 * first.market_price) / LAMPORTS_PER_SOL_F64;
            // Final value: Token B + Token A * market price
            let final_value = (final_b as f64 + final_a as f64 * last.market_price) / LAMPORTS_PER_SOL_F64;
            final_value - initial_value
        }
        _ => 0.0,
    }
}

/// Calculate total profit of arbitrageurs
fn calculate_arb_profit(first: &ReservesRecord, last: &ReservesRecord) -> f64 {
    match (first.arb_balance_a, first.arb_balance_b, last.arb_balance_a, last.arb_balance_b) {
        (Some(initial_a), Some(initial_b), Some(final_a), Some(final_b)) => {
            let initial_value = (initial_b as f64 + initial_a as f64 * first.market_price) / LAMPORTS_PER_SOL_F64;
            let final_value = (final_b as f64 + final_a as f64 * last.market_price) / LAMPORTS_PER_SOL_F64;
            final_value - initial_value
        }
        _ => 0.0,
    }
}

/// Calculate nominal LP profit (LP Profit Value)
fn calculate_lp_profit_value(first: &ReservesRecord, last: &ReservesRecord) -> f64 {
    let calc_tvl = |record: &ReservesRecord| -> f64 {
        let total_a: u64 = record.vault_reserves.iter().map(|(a, _)| a).sum();
        let total_b: u64 = record.vault_reserves.iter().map(|(_, b)| b).sum();
        (total_b as f64 + total_a as f64 * record.market_price) / LAMPORTS_PER_SOL_F64
    };
    
    let initial_tvl = calc_tvl(first);
    let final_tvl = calc_tvl(last);
    final_tvl - initial_tvl
}

/// Calculate number of arbitrage trades (changes in arbitrageur balance)
fn count_arb_trades(reserves: &[ReservesRecord]) -> u64 {
    let mut count = 0;
    
    for i in 1..reserves.len() {
        let prev = &reserves[i - 1];
        let curr = &reserves[i];
        
        if let (Some(prev_a), Some(curr_a), Some(prev_b), Some(curr_b)) = 
            (prev.arb_balance_a, curr.arb_balance_a, prev.arb_balance_b, curr.arb_balance_b) {
            // Jeśli którykolwiek balans się zmienił, to była transakcja
            if prev_a != curr_a || prev_b != curr_b {
                count += 1;
            }
        }
    }
    
    count
}

/// Calculate average and maximum spread in percentage
fn calculate_spreads(reserves: &[ReservesRecord]) -> (f64, f64) {
    let mut spreads_pct = Vec::new();
    
    for record in reserves {
        let prices = record.vault_prices();
        
        if prices.is_empty() || prices.iter().any(|p| !p.is_finite() || *p <= 0.0) {
            continue;
        }
        
        let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        // Spread w procentach: (max - min) / min * 100
        if min_price > 0.0 {
            let spread_pct = ((max_price - min_price) / min_price) * 100.0;
            spreads_pct.push(spread_pct);
        }
    }
    
    if spreads_pct.is_empty() {
        return (0.0, 0.0);
    }
    
    let avg_spread = spreads_pct.iter().sum::<f64>() / spreads_pct.len() as f64;
    let max_spread = spreads_pct.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    (avg_spread, max_spread)
}

/// Calculate RMSE of vault prices from market price
fn calculate_rmse_price(reserves: &[ReservesRecord]) -> f64 {
    let mut squared_errors = Vec::new();
    
    for record in reserves {
        let prices = record.vault_prices();
        let market_price = record.market_price;
        
        for price in prices {
            if price.is_finite() && price > 0.0 {
                let error = price - market_price;
                squared_errors.push(error * error);
            }
        }
    }
    
    if squared_errors.is_empty() {
        return 0.0;
    }
    
    let mse = squared_errors.iter().sum::<f64>() / squared_errors.len() as f64;
    mse.sqrt()
}

/// Calculate Impermanent Loss in percentage
/// 
/// IL measures the opportunity cost of providing liquidity vs holding tokens.
/// Positive IL means loss (HODL would be better), negative means gain (LP fees exceeded IL).
/// 
/// Formula: IL = (HODL_value - LP_value) / HODL_value * 100
/// where both values are priced at final market price
fn calculate_impermanent_loss(first: &ReservesRecord, last: &ReservesRecord) -> f64 {
    // Initial pool reserves
    let initial_total_a: u64 = first.vault_reserves.iter().map(|(a, _)| a).sum();
    let initial_total_b: u64 = first.vault_reserves.iter().map(|(_, b)| b).sum();
    
    // Final pool reserves (includes fees earned)
    let final_total_a: u64 = last.vault_reserves.iter().map(|(a, _)| a).sum();
    let final_total_b: u64 = last.vault_reserves.iter().map(|(_, b)| b).sum();
    
    // HODL value: what initial tokens would be worth at final market price
    let hodl_value = (initial_total_b as f64 + initial_total_a as f64 * last.market_price) / LAMPORTS_PER_SOL_F64;
    
    // LP value: what pool tokens are worth at final market price (includes fees)
    let lp_value = (final_total_b as f64 + final_total_a as f64 * last.market_price) / LAMPORTS_PER_SOL_F64;
    
    if hodl_value == 0.0 {
        return 0.0;
    }
    
    // IL = (HODL - LP) / HODL * 100
    // Positive: loss from IL (HODL better)
    // Negative: gain from fees exceeding IL (LP better)
    ((hodl_value - lp_value) / hodl_value) * 100.0
}

use anyhow::Result;
use csv::Reader;
use std::path::Path;
use crate::types::{ReservesRecord, ComputeRecord};

pub fn load_reserves_csv<P: AsRef<Path>>(path: P) -> Result<Vec<ReservesRecord>> {
    let mut reader = Reader::from_path(path)?;
    let mut records = Vec::new();

    // Get headers to determine vault count
    let headers = reader.headers()?.clone();
    
    for result in reader.records() {
        let record = result?;
        
        // Parse slot and market_price
        let slot: u64 = record.get(1).unwrap_or("0").parse()?;
        let market_price: f64 = record.get(2).unwrap_or("0.0").parse()?;
        
        // Dynamically parse all vault reserves
        let mut vault_reserves = Vec::new();
        let mut vault_idx = 0;
        
        loop {
            let reserve_a_col = format!("vault_{}_reserve_a", vault_idx);
            let reserve_b_col = format!("vault_{}_reserve_b", vault_idx);
            
            let a_pos = headers.iter().position(|h| h == reserve_a_col);
            let b_pos = headers.iter().position(|h| h == reserve_b_col);
            
            match (a_pos, b_pos) {
                (Some(a_idx), Some(b_idx)) => {
                    let reserve_a: u64 = record.get(a_idx).unwrap_or("0").parse().unwrap_or(0);
                    let reserve_b: u64 = record.get(b_idx).unwrap_or("0").parse().unwrap_or(0);
                    vault_reserves.push((reserve_a, reserve_b));
                    vault_idx += 1;
                }
                _ => break,
            }
        }
        
        // Parse optional balances
        let arb_a_pos = headers.iter().position(|h| h == "arb_balance_a");
        let arb_b_pos = headers.iter().position(|h| h == "arb_balance_b");
        let user_a_pos = headers.iter().position(|h| h == "users_balance_a" || h == "user_balance_a");
        let user_b_pos = headers.iter().position(|h| h == "users_balance_b" || h == "user_balance_b");
        
        let arb_balance_a = arb_a_pos.and_then(|idx| record.get(idx).and_then(|s| s.parse().ok()));
        let arb_balance_b = arb_b_pos.and_then(|idx| record.get(idx).and_then(|s| s.parse().ok()));
        let user_balance_a = user_a_pos.and_then(|idx| record.get(idx).and_then(|s| s.parse().ok()));
        let user_balance_b = user_b_pos.and_then(|idx| record.get(idx).and_then(|s| s.parse().ok()));
        
        records.push(ReservesRecord {
            _slot: slot,
            market_price,
            vault_reserves,
            arb_balance_a,
            arb_balance_b,
            user_balance_a,
            user_balance_b,
        });
    }

    Ok(records)
}

pub fn load_compute_csv<P: AsRef<Path>>(path: P) -> Result<Vec<ComputeRecord>> {
    let mut reader = Reader::from_path(path)?;
    let mut records = Vec::new();

    for result in reader.deserialize() {
        let record: ComputeRecord = result?;
        records.push(record);
    }

    Ok(records)
}

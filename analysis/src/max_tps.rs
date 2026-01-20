use std::collections::HashMap;
use std::fs;
use std::path::Path;
use csv::ReaderBuilder;

use crate::{AnalysisArgs, BASE_DIRS};
use crate::types::ComputeRecord;

#[derive(Debug)]
struct VaultStats {
    max_tx_per_slot: u64,
    total_tx: u64,
    total_slots: u64,
}

fn process_dual_file(file_path: &Path, tx_counter: &mut usize) -> Result<HashMap<(u64, u64), VaultStats>, Box<dyn std::error::Error>> {
    let mut vault_pair_stats: HashMap<(u64, u64), HashMap<u64, u64>> = HashMap::new();
    
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(file_path)?;
    
    for result in rdr.deserialize() {
        let record: ComputeRecord = result?;
        
        if record.success != 1 {
            continue;
        }
        
        if let Some(vault_2) = record.vault_2 {
            let vault_pair = if record.vault_1 < vault_2 {
                (record.vault_1, vault_2)
            } else {
                (vault_2, record.vault_1)
            };
            
            let slot_counts = vault_pair_stats.entry(vault_pair).or_insert_with(HashMap::new);
            *slot_counts.entry(record.slot).or_insert(0) += 1;
            *tx_counter += 1;
        }
    }
    
    let mut result = HashMap::new();
    for (vault_pair, slot_counts) in vault_pair_stats {
        let max_tx_per_slot = *slot_counts.values().max().unwrap_or(&0);
        let total_tx = slot_counts.values().sum();
        let total_slots = slot_counts.len() as u64;
        
        result.insert(vault_pair, VaultStats {
            max_tx_per_slot,
            total_tx,
            total_slots,
        });
    }
    
    Ok(result)
}

fn process_single_file(file_path: &Path, tx_counter: &mut usize) -> Result<HashMap<u64, VaultStats>, Box<dyn std::error::Error>> {
    let mut vault_stats: HashMap<u64, HashMap<u64, u64>> = HashMap::new();
    
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(file_path)?;
    
    for result in rdr.deserialize() {
        let record: ComputeRecord = result?;
        
        if record.success != 1 {
            continue;
        }
        
        let slot_counts = vault_stats.entry(record.vault_1).or_insert_with(HashMap::new);
        *slot_counts.entry(record.slot).or_insert(0) += 1;
        *tx_counter += 1;
    }
    
    let mut result = HashMap::new();
    for (vault, slot_counts) in vault_stats {
        let max_tx_per_slot = *slot_counts.values().max().unwrap_or(&0);
        let total_tx = slot_counts.values().sum();
        let total_slots = slot_counts.len() as u64;
        
        result.insert(vault, VaultStats {
            max_tx_per_slot,
            total_tx,
            total_slots,
        });
    }
    
    Ok(result)
}

pub fn run_max_tps_analysis(_arg: &AnalysisArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx_counter = 0usize;
    
    for base_dir in &BASE_DIRS {
        println!("\n{}", "=".repeat(80));
        println!("Processing directory: {}", base_dir);
        println!("{}\n", "=".repeat(80));
        
        // Process DUAL swaps
        let dual_dir = Path::new(&base_dir.to_string()).join("dual");
        if dual_dir.exists() {
            println!("DUAL SWAPS:");
            println!("{}", "-".repeat(80));
            
            let mut all_vault_pairs: HashMap<(u64, u64), VaultStats> = HashMap::new();
            
            if let Ok(entries) = fs::read_dir(&dual_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && 
                           path.file_name().unwrap().to_str().unwrap().starts_with("seed_") &&
                           path.file_name().unwrap().to_str().unwrap().ends_with("_compute.csv") {
                            
                            if let Ok(file_stats) = process_dual_file(&path, &mut tx_counter) {
                                for (vault_pair, stats) in file_stats {
                                    let entry = all_vault_pairs.entry(vault_pair).or_insert(VaultStats {
                                        max_tx_per_slot: 0,
                                        total_tx: 0,
                                        total_slots: 0,
                                    });
                                    
                                    entry.max_tx_per_slot = entry.max_tx_per_slot.max(stats.max_tx_per_slot);
                                    entry.total_tx += stats.total_tx;
                                    entry.total_slots += stats.total_slots;
                                }
                            }
                        }
                    }
                }
            }
            
            let mut vault_pairs: Vec<_> = all_vault_pairs.iter().collect();
            vault_pairs.sort_by_key(|(pair, _)| *pair);
            
            for ((vault_1, vault_2), stats) in vault_pairs {
                println!("Vault pair ({}, {}): Max TX/slot = {}, Total TX = {}, Total slots = {}", 
                    vault_1, vault_2, stats.max_tx_per_slot, stats.total_tx, stats.total_slots);
            }
            
            if let Some(max_stat) = all_vault_pairs.values().max_by_key(|s| s.max_tx_per_slot) {
                println!("\nOverall Max TX/slot for DUAL swaps: {}", max_stat.max_tx_per_slot);
            }
        }
        
        // Process SINGLE swaps
        let single_dir = Path::new(&base_dir.to_string()).join("single");
        if single_dir.exists() {
            println!("\n\nSINGLE SWAPS:");
            println!("{}", "-".repeat(80));
            
            let mut all_vaults: HashMap<u64, VaultStats> = HashMap::new();
            
            if let Ok(entries) = fs::read_dir(&single_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && 
                           path.file_name().unwrap().to_str().unwrap().starts_with("seed_") &&
                           path.file_name().unwrap().to_str().unwrap().ends_with("_compute.csv") {
                            
                            if let Ok(file_stats) = process_single_file(&path, &mut tx_counter) {
                                for (vault, stats) in file_stats {
                                    let entry = all_vaults.entry(vault).or_insert(VaultStats {
                                        max_tx_per_slot: 0,
                                        total_tx: 0,
                                        total_slots: 0,
                                    });
                                    
                                    entry.max_tx_per_slot = entry.max_tx_per_slot.max(stats.max_tx_per_slot);
                                    entry.total_tx += stats.total_tx;
                                    entry.total_slots += stats.total_slots;
                                }
                            }
                        }
                    }
                }
            }
            
            let mut vaults: Vec<_> = all_vaults.iter().collect();
            vaults.sort_by_key(|(vault, _)| *vault);
            
            for (vault, stats) in vaults {
                println!("Vault {}: Max TX/slot = {}, Total TX = {}, Total slots = {}", 
                    vault, stats.max_tx_per_slot, stats.total_tx, stats.total_slots);
            }
            
            if let Some(max_stat) = all_vaults.values().max_by_key(|s| s.max_tx_per_slot) {
                println!("\nOverall Max TX/slot for SINGLE swaps: {}", max_stat.max_tx_per_slot);
            }
        }
    }
    println!("Total transactions processed: {}", tx_counter);
    Ok(())
}

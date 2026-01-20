use anyhow::{Context, Result};
use csv::Writer;
use std::fs::File;
use tracing::{debug, info};

use crate::utils::{enviroment::simulation::FLUSH_INTERVAL_SLOTS};

pub struct CsvWriter {
    writer: Writer<File>,
    transaction_count: u64,
}

pub struct SwapComputeCsvWriter {
    writer: Writer<File>,
    swap_count: u64,
    buffer: Vec<SwapComputeRecord>,
}

#[derive(Clone)]
struct SwapComputeRecord {
    slot: u64,
    success: bool,
    vault_1: Option<u8>,
    vault_2: Option<u8>,
    is_a_to_b: bool,
    amount_in: u64,
    amount_out: u64,
    compute_units: u64,
}

impl CsvWriter {
    pub fn new(file_path: &str, vault_count: u8) -> Result<Self> {
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create CSV file: {}", file_path))?;

        let mut writer = Writer::from_writer(file);

        // Write header
        let mut headers = vec![
            "transaction_id".to_string(),
            "slot".to_string(),
            "market_price".to_string(),
        ];
        for i in 0..vault_count {
            headers.push(format!("vault_{}_reserve_a", i));
            headers.push(format!("vault_{}_reserve_b", i));
            headers.push(format!("vault_{}_price", i));
        }

        // Arbitrageur balances (sum across all arbitrage bots)
        headers.push("arb_balance_a".to_string());
        headers.push("arb_balance_b".to_string());

        headers.push("users_balance_a".to_string());
        headers.push("users_balance_b".to_string());

        writer
            .write_record(&headers)
            .context("Failed to write CSV header")?;

        info!("CSV writer initialized: {}", file_path);

        Ok(Self {
            writer,
            transaction_count: 0,
        })
    }

    pub fn write_reserves(
        &mut self,
        slot: u64,
        market_price: f64,
        vault_reserves: &[(u64, u64)],
        arbitrageur_balances: Option<(u64, u64)>,
        user_total_balances: Option<(u64, u64)>,
    ) -> Result<()> {
        self.transaction_count += 1;

        let mut record = vec![
            self.transaction_count.to_string(),
            slot.to_string(),
            format!("{:.12}", market_price),
        ];

        for (reserve_a, reserve_b) in vault_reserves {
            record.push(reserve_a.to_string());
            record.push(reserve_b.to_string());

            // Spot price in the vault, expressed as B per 1 A.
            let vault_price = if *reserve_a == 0 {
                0.0
            } else {
                *reserve_b as f64 / *reserve_a as f64
            };
            record.push(format!("{:.12}", vault_price));
        }

        let (arb_a, arb_b) = arbitrageur_balances.unwrap_or((0, 0));
        record.push(arb_a.to_string());
        record.push(arb_b.to_string());

        let (user_a, user_b) = user_total_balances.unwrap_or((0, 0));
        record.push(user_a.to_string());
        record.push(user_b.to_string());

        self.writer
            .write_record(&record)
            .context("Failed to write CSV record")?;

        // Periodic flush for real-time monitoring
        if self.transaction_count % 100 == 0 {
            self.writer.flush().context("Failed to flush CSV writer")?;
            debug!("Flushed CSV at transaction {}", self.transaction_count);
        }

        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        self.writer
            .flush()
            .context("Failed to finalize CSV writer")?;
        info!(
            "CSV writer finalized with {} slots",
            self.transaction_count
        );
        Ok(())
    }

    pub fn transaction_count(&self) -> u64 {
        self.transaction_count
    }
}

impl SwapComputeCsvWriter {
    pub fn new(file_path: &str, user_count: usize, bot: u8, frequency: f64) -> Result<Self> {
        let file = File::create(file_path)
            .with_context(|| format!("Failed to create compute CSV file: {}", file_path))?;

        let mut writer = Writer::from_writer(file);

        writer
            .write_record([
                "swap_id",
                "slot",
                "success",
                "vault_1",
                "vault_2",
                "is_a_to_b",
                "amount_in",
                "amount_out",
                "compute_units",
            ])
            .context("Failed to write compute CSV header")?;

        info!("Compute CSV writer initialized: {}", file_path);

        let posibly = (user_count + bot as usize) as f64 * frequency;
        Ok(Self {
            writer,
            swap_count: 0,
            buffer: Vec::with_capacity((posibly * FLUSH_INTERVAL_SLOTS as f64) as usize), // ~100k swaps per flush (user_count * 5% * 1000 slots * margin)
        })
    }

    pub fn write_swap(
        &mut self,
        slot: u64,
        success: bool,
        vault_1: Option<u8>,
        vault_2: Option<u8>,
        is_a_to_b: bool,
        amount_in: u64,
        amount_out: u64,
        compute_units: u64,
    ) -> Result<()> {
        // Add to buffer instead of writing immediately
        self.buffer.push(SwapComputeRecord {
            slot,
            success,
            vault_1,
            vault_2,
            is_a_to_b,
            amount_in,
            amount_out,
            compute_units,
        });

        Ok(())
    }

    pub fn flush_buffer(&mut self) -> Result<()> {
        for record in &self.buffer {
            self.swap_count += 1;

            let csv_record = [
                self.swap_count.to_string(),
                record.slot.to_string(),
                (record.success as u8).to_string(),
                record.vault_1.map(|v| v.to_string()).unwrap_or_default(),
                record.vault_2.map(|v| v.to_string()).unwrap_or_default(),
                (record.is_a_to_b as u8).to_string(),
                record.amount_in.to_string(),
                record.amount_out.to_string(),
                record.compute_units.to_string(),
            ];

            self.writer
                .write_record(csv_record)
                .context("Failed to write compute CSV record")?;
        }

        self.buffer.clear();
        self.writer.flush().context("Failed to flush compute CSV writer")?;
        debug!("Flushed compute CSV buffer with {} swaps", self.swap_count);

        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        // Flush any remaining buffered data
        if !self.buffer.is_empty() {
            self.flush_buffer()?;
        }
        
        self.writer
            .flush()
            .context("Failed to finalize compute CSV writer")?;
        info!(
            "✅ Compute CSV writer finalized with {} swaps",
            self.swap_count
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_writer_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let writer = CsvWriter::new(path, 3);
        assert!(writer.is_ok());
    }

    #[test]
    fn test_write_reserves() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut writer = CsvWriter::new(path, 2).unwrap();
        let reserves = vec![(1000, 2000), (3000, 4000)];

        assert!(writer
            .write_reserves(1, 1.2345, &reserves, Some((50, 60)), Some((10, 20)))
            .is_ok());
        assert_eq!(writer.transaction_count(), 1);
    }

    #[test]
    fn test_compute_writer() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut writer = SwapComputeCsvWriter::new(path, 2, 0, 0.05).unwrap();
        writer
            .write_swap(1, true, Some(0), None, true, 100, 99, 12_345)
            .unwrap();
    }
}

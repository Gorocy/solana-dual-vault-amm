use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct ReservesRecord {
    pub _slot: u64,
    pub market_price: f64,
    pub vault_reserves: Vec<(u64, u64)>, // Vec of (reserve_a, reserve_b) for each vault
    pub arb_balance_a: Option<u64>,
    pub arb_balance_b: Option<u64>,
    pub user_balance_a: Option<u64>,
    pub user_balance_b: Option<u64>,
}

impl ReservesRecord {

    pub fn vault_prices(&self) -> Vec<f64> {
        self.vault_reserves
            .iter()
            .map(|(a, b)| {
                if *a == 0 {
                    0.0
                } else {
                    *b as f64 / *a as f64
                }
            })
            .collect()
    }
}
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct ComputeRecord {
    pub swap_id: u64,
    pub slot: u64,
    pub success: u8,  // 1 = success, 0 = failure
    pub vault_1: u64,
    #[serde(deserialize_with = "deserialize_optional_u64")]
    pub vault_2: Option<u64>,  // Empty string for single vault swaps
    pub is_a_to_b: u8,  // 1 = true, 0 = false
    pub amount_in: u64,
    pub amount_out: u64,
    pub compute_units: u64,
}

// Custom deserializer for empty string -> None
fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        s.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

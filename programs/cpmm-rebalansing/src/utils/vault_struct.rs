pub struct VaultAssets {
    pub incoming: u128,
    pub outgoing: u128,
}

impl VaultAssets {
    pub fn new(incoming: u64, outgoing: u64) -> Self {
        Self {
            incoming: incoming as u128,
            outgoing: outgoing as u128,
        }
    }
}

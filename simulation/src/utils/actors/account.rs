use litesvm::LiteSVM;
use rand::{rngs::StdRng, Rng, SeedableRng};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{SeedDerivable, Signer},
};

use crate::{
    error::{ResultSimulation, SimulationError},
    utils::ix::{context::SimContext, get_token_balance, token_utils::TokenUtils},
};

#[derive(Debug)]
pub struct SimUser {
    pub keypair: Keypair,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
}

impl Clone for SimUser {
    fn clone(&self) -> Self {
        Self {
            keypair: self.keypair.insecure_clone(),
            token_a: self.token_a,
            token_b: self.token_b,
        }
    }
}

impl SimUser {
    pub fn new(
        ctx: &mut SimContext,
        mint_a: Pubkey,
        mint_b: Pubkey,
        initial_balance_a: u64,
        initial_balance_b: u64,
        seed: u64,
    ) -> ResultSimulation<Self> {
        let mut rng = StdRng::seed_from_u64(seed);

        let mut seed_bytes = [0u8; 32];
        rng.fill(&mut seed_bytes);

        let keypair = Keypair::from_seed(&seed_bytes)
            .map_err(|_| SimulationError::SystemError("Keypair from seed".into()))?;

        ctx.airdrop(&keypair.pubkey(), 100 * LAMPORTS_PER_SOL)?;

        let token_a =
            TokenUtils::create_token_account(ctx, &mint_a, keypair.pubkey(), initial_balance_a)?;

        let token_b =
            TokenUtils::create_token_account(ctx, &mint_b, keypair.pubkey(), initial_balance_b)?;

        Ok(Self {
            keypair,
            token_a,
            token_b,
        })
    }
    /// Pobiera balanse usera bezpośrednio z SVM
    pub fn get_balance_a(&self, svm: &LiteSVM) -> u64 {
        get_token_balance(svm, self.token_a).unwrap_or(0)
    }

    /// Pobiera balanse usera bezpośrednio z SVM
    pub fn get_balance_b(&self, svm: &LiteSVM) -> u64 {
        get_token_balance(svm, self.token_b).unwrap_or(0)
    }

    pub fn get_balances(&self, svm: &LiteSVM) -> (u64, u64) {
        (self.get_balance_a(svm), self.get_balance_b(svm))
    }

    /// Rebalances portfolio to 50/50 value split by trading on external market.
    ///
    /// This simulates what real arbitrageurs do: immediately after executing arbitrage
    /// on the CPMM, they trade on a liquid external market (e.g., CEX, aggregator) to
    /// return their portfolio to a neutral 50/50 value position.
    ///
    /// # Fee Assumptions
    ///
    /// **No explicit fee is deducted** in this simulation. This is intentional and based
    /// on the following assumptions:
    ///
    /// 1. **External market volatility**: We assume the external market is sufficiently
    ///    liquid and volatile that the actual market price already deviates from the
    ///    theoretical price by more than typical trading fees (0.1-0.3%).
    ///
    /// 2. **Market microstructure**: Real-world external markets exhibit bid-ask spreads,
    ///    price slippage, and temporary price dislocations that effectively "contain" the
    ///    fee cost within normal market noise.
    ///
    /// 3. **Conservative arbitrage**: Arbitrageurs only execute when profit exceeds their
    ///    configured threshold (`min_profit_threshold`), which should already account for
    ///    all-in costs including external market fees.
    ///
    /// In essence, the external market is "noisy enough" that the fee becomes negligible
    /// compared to the price deviation that enabled the arbitrage in the first place.
    ///
    /// # Parameters
    ///
    /// * `ctx` - Simulation context for executing balance adjustments
    /// * `market_price` - External market price (token B per token A)
    ///
    /// # Returns
    ///
    /// Tuple of `(trade_a, trade_b)` where:
    /// * Positive values indicate tokens bought
    /// * Negative values indicate tokens sold
    pub fn rebalance_to_5050(
        &self,
        ctx: &mut SimContext,
        market_price: f64,
    ) -> ResultSimulation<(i64, i64)> {
        let (balance_a, balance_b) = self.get_balances(&ctx.svm);

        // Calculate current value in terms of token A
        let value_a = balance_a as f64;
        let value_b_in_a = (balance_b as f64) / market_price;
        let total_value_in_a = value_a + value_b_in_a;

        // Target: 50% of value in each token
        let target_a = total_value_in_a / 2.0;
        let target_b_in_a = total_value_in_a / 2.0;
        let target_b = target_b_in_a * market_price;

        // Calculate what we need to trade
        let delta_a = target_a - value_a;
        let delta_b = target_b - (balance_b as f64);

        // Execute the rebalancing trades
        // Note: In real implementation, this would be actual token transfers
        // For simulation purposes, we directly adjust balances
        let trade_a = delta_a as i64;
        let trade_b = delta_b as i64;

        if trade_a.abs() > 0 {
            TokenUtils::adjust_token_balance(
                ctx,
                self.token_a,
                trade_a,
            )?;
        }

        if trade_b.abs() > 0 {
            TokenUtils::adjust_token_balance(
                ctx,
                self.token_b,
                trade_b,
            )?;
        }

        Ok((trade_a, trade_b))
    }
}

#[derive(Clone, Copy)]
pub struct ProfitCalculator {
    market_price: f64,
}

impl ProfitCalculator {
    pub fn new(market_price: f64) -> Self {
        Self { market_price }
    }

    pub fn profit_evaluator_a_to_b<F>(
        self,
        calculate_output: F,
    ) -> impl Fn(u64) -> Option<(u64, f64)>
    where
        F: Fn(u64) -> Option<u64>,
    {
        let market_price = self.market_price;
        move |amount_in: u64| {
            let expected_output = calculate_output(amount_in)?;
            // For A→B swap:
            // - expected_output is in token B
            // - amount_in is in token A
            // - Need to convert expected_output from B to A value terms: B / market_price
            // - market_price is defined as: 1 token A = market_price tokens B
            // - Therefore: B / market_price = equivalent value in A
            let expected_value_in_a = expected_output as f64 / market_price;
            let net_profit = expected_value_in_a - amount_in as f64;
            if net_profit > 0.0 {
                Some((expected_output, net_profit))
            } else {
                None
            }
        }
    }

    pub fn profit_evaluator_b_to_a<F>(
        self,
        calculate_output: F,
    ) -> impl Fn(u64) -> Option<(u64, f64)>
    where
        F: Fn(u64) -> Option<u64>,
    {
        let market_price = self.market_price;
        move |amount_in: u64| {
            let expected_output = calculate_output(amount_in)?;
            // For B→A swap:
            // - expected_output is in token A
            // - amount_in is in token B
            // - Need to convert amount_in from B to A value terms: B * (1/market_price)
            // - market_price is defined as: 1 token A = market_price tokens B
            // - Therefore: B / market_price = equivalent value in A
            let amount_in_value_a = amount_in as f64 / market_price;
            let net_profit = (expected_output as f64) - amount_in_value_a;
            if net_profit > 0.0 {
                Some((expected_output, net_profit))
            } else {
                None
            }
        }
    }
}

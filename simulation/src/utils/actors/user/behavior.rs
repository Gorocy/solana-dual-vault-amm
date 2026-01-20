use crate::utils::actors::SwapStrategy;

#[derive(Debug, Clone)]
pub struct UserBehaviorConfigBuilder {
    pub min_swap_amount_percentage: f64,
    pub max_swap_amount_percentage: f64,
    pub swap_frequency: f64,
    pub a_to_b_probability: f64,
}

#[derive(Debug, Clone)]
pub struct UserBehaviorConfig {
    /// Minimalny % balance do swap (np. 0.01 = 1%)
    pub min_swap_amount_percentage: f64,
    /// Maksymalny % balance do swap (np. 0.10 = 10%)
    pub max_swap_amount_percentage: f64,
    /// Prawdopodobieństwo wykonania swap w danej rundzie
    pub swap_frequency: f64,
    /// Prawdopodobieństwo swap A->B vs B->A
    pub a_to_b_probability: f64,
    /// Strategia swapu (SingleVault, DualVault)
    pub strategy: SwapStrategy,
}

impl UserBehaviorConfig {
    pub fn build(builder: UserBehaviorConfigBuilder, strategy: SwapStrategy) -> Self {
        Self {
            min_swap_amount_percentage: builder.min_swap_amount_percentage,
            max_swap_amount_percentage: builder.max_swap_amount_percentage,
            swap_frequency: builder.swap_frequency,
            a_to_b_probability: builder.a_to_b_probability,
            strategy,
        }
    }
}

use anyhow::Result;

use crate::utils::{
    actors::{
        arbitrageur_bot::{
            dual::bot::DualArbitrageBot,
            manager::{ArbitrageManager, ManagedArbitrageBot},
            single::bot::SingleArbitrageBot,
            traits::ArbitrageBotFactory,
            ArbitrageConfig,
        },
        user::user_manager::UserManager,
        SwapStrategy,
    },
    args::{config_simulation::SimulationConfig, SEED_OFFSET_USER_MANAGER},
};

use super::{
    env_driver::SimulationEnvDriver,
    manager_driver::SharedStateUpdater,
    setup::{EnvMode, MarketSimulationEnvironment},
    simulation::Simulation,
    utils::{create_users, create_writers, user_behavior},
};

/// Generic simulation builder
fn new_simulation<B>(
    mode: EnvMode,
    swap_strategy: SwapStrategy,
    config: SimulationConfig,
    balance_picker: impl Fn(u64, u64, u8) -> (u64, u64),
) -> Result<Simulation<ArbitrageManager<B>>>
where
    B: ArbitrageBotFactory + ManagedArbitrageBot<Environment = MarketSimulationEnvironment>,
    B::Environment: SimulationEnvDriver,
    B::SharedState: SharedStateUpdater + Default,
{
    let arbitrage_config = ArbitrageConfig {
        min_profit_threshold: config.arb_min_profit_threshold,
        max_position_size: config.arb_max_position_size,
        check_frequency: config.arb_check_frequency,
        min_swap_amount: config.arb_min_swap_amount,
        search_precision_ratio: config.arb_search_precision_ratio,
    };

    let (arbitrage_manager, mut env) = if config.enable_arbitrageurs {
        crate::utils::enviroment::setup::setup_market_environment_generic::<B, _>(
            mode,
            config.vault_count,
            config.initial_liquidity_a,
            config.initial_liquidity_b,
            config.market_volatility,
            config.market_drift,
            config.simulation_seed,
            config.arbitrageur_count,
            arbitrage_config.clone(),
            config.fee_option.clone(),
            balance_picker,
        )?
    } else {
        crate::utils::enviroment::setup::setup_market_environment_generic::<B, _>(
            mode,
            config.vault_count,
            config.initial_liquidity_a,
            config.initial_liquidity_b,
            config.market_volatility,
            config.market_drift,
            config.simulation_seed,
            0,
            arbitrage_config,
            config.fee_option.clone(),
            balance_picker,
        )?
    };

    let users = create_users(&config, &mut env.ctx, env.mint_a, env.mint_b);
    let user_behavior_config = user_behavior(&config, swap_strategy);
    let user_manager = UserManager::new(users, user_behavior_config, config.simulation_seed.wrapping_add(SEED_OFFSET_USER_MANAGER));

    let (csv_writer, swap_compute_writer) = create_writers(&config)?;

    Ok(Simulation::new(
        env,
        user_manager,
        arbitrage_manager,
        csv_writer,
        swap_compute_writer,
        config,
    ))
}

/// Build a single-vault simulation
pub fn new_single_simulation(
    mut config: SimulationConfig,
) -> Result<Simulation<ArbitrageManager<SingleArbitrageBot>>> {
    config.strategy = 1;
    // Single vault has 2x liquidity to compensate for not having 2x vaults
    config.initial_liquidity_a *= 2;
    config.initial_liquidity_b *= 2;
    config.validate()?;

    new_simulation::<SingleArbitrageBot>(
        EnvMode::Single,
        SwapStrategy::SingleVault,
        config,
        |a: u64, b: u64, v: u8| (a*v as u64, b*v as u64),
    )
}

/// Build a dual-vault simulation
pub fn new_dual_simulation(
    mut config: SimulationConfig,
) -> Result<Simulation<ArbitrageManager<DualArbitrageBot>>> {
    config.strategy = 2;
    // Dual vault has 2x the number of vaults
    config.vault_count *= 2;
    config.validate()?;

    new_simulation::<DualArbitrageBot>(
        EnvMode::Dual,
        SwapStrategy::DualVault,
        config,
        |a: u64, b: u64, v: u8| (a*v as u64, b*v as u64),
    )
}

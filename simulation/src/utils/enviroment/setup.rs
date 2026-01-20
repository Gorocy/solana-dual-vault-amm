use crate::utils::actors::arbitrageur_bot::{
    manager::{
        add_generic_arbitrageur, new_generic_manager, ArbitrageManager, ManagedArbitrageBot,
    },
    traits::ArbitrageBotFactory,
    ArbitrageConfig,
};
use crate::utils::args::{SEED_OFFSET_ARBITRAGEUR_BASE, SEED_OFFSET_MARKET_PRICE};
use crate::utils::ix::{
    context::SimContext, manage_liqudity::ManageLiquidityInstruction,
    registry_utils::RegistryUtils, token_utils::TokenUtils, vault_utils::VaultUtils, FeeOption,
};
use crate::utils::market::{MarketPriceGenerator, SOLANA_BLOCK_TIME};
use anyhow::Result;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signer::Signer};

pub struct SimulationEnvironment {
    pub ctx: SimContext,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub registry: Pubkey,
    pub vaults: Vec<Pubkey>,
}

pub struct MarketSetupBase {
    pub ctx: SimContext,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub registry: Pubkey,
    pub vaults: Vec<Pubkey>,
    pub market: MarketPriceGenerator,
    pub initial_price: f64,
    pub fee_rate: FeeOption,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvMode {
    Single,
    Dual,
}

pub struct MarketSimulationEnvironment {
    pub ctx: SimContext,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub registry: Pubkey,
    pub vaults: Vec<Pubkey>,
    pub market: MarketPriceGenerator,
    pub initial_price: f64,
    pub fee_rate: FeeOption,
    pub mode: EnvMode,
}

pub fn setup_environment(vault_count: u8, fee: FeeOption) -> Result<SimulationEnvironment> {
    // ─────────────────────────────────────────────
    // Context + deploy
    // ─────────────────────────────────────────────
    let mut ctx = SimContext::new();

    ctx.airdrop_payer(1000 * LAMPORTS_PER_SOL)?;

    ctx.deploy_program("cpmm_rebalansing", crate::PROGRAM_ID);

    // ─────────────────────────────────────────────
    // Minty
    // ─────────────────────────────────────────────
    let pair = TokenUtils::setup_sorted_mints(&mut ctx);

    let mint_a = pair.mint_a;
    let mint_b = pair.mint_b;

    // ─────────────────────────────────────────────
    // Vault Registry
    // ─────────────────────────────────────────────
    let registry = RegistryUtils::initialize_vault_registry(&mut ctx, mint_a, mint_b, fee)?;

    // ─────────────────────────────────────────────
    // Vaulty
    // ─────────────────────────────────────────────
    let mut vaults = Vec::new();

    for _ in 0..vault_count {
        let vault = VaultUtils::init_vault(&mut ctx, crate::PROGRAM_ID, registry)?;
        vaults.push(vault);
    }

    Ok(SimulationEnvironment {
        ctx,
        mint_a,
        mint_b,
        registry,
        vaults,
    })
}

pub fn setup_market_base(
    vault_count: u8,
    deposit_a: u64,
    deposit_b: u64,
    market_volatility: f64,
    market_drift: f64,
    seed: u64,
    fee: FeeOption,
    payer_liquidity_multiplier: u64,
) -> Result<MarketSetupBase> {
    let SimulationEnvironment {
        mut ctx,
        mint_a,
        mint_b,
        registry,
        vaults,
        ..
    } = setup_environment(vault_count, fee.clone())?;

    let payer = ctx.payer.insecure_clone().pubkey();

    TokenUtils::create_token_account(
        &mut ctx,
        &mint_a,
        payer,
        deposit_a * payer_liquidity_multiplier,
    )?;

    TokenUtils::create_token_account(
        &mut ctx,
        &mint_b,
        payer,
        deposit_b * payer_liquidity_multiplier,
    )?;

    for (i, _) in vaults.iter().enumerate() {
        ManageLiquidityInstruction::add_liquidity_to_vault(
            &mut ctx, None, registry, i as u8, deposit_a, deposit_b, 0,
        )?;
    }

    let initial_price = deposit_b as f64 / deposit_a as f64;

    // Use Solana block time (0.4s) for realistic crypto market price movements
    // Each call to market.next_price() simulates price change over one Solana block
    // using Geometric Brownian Motion (GBM) with the specified volatility and drift
    let dt = MarketPriceGenerator::dt_from_seconds(SOLANA_BLOCK_TIME);
    
    let market = MarketPriceGenerator::new(
        initial_price,
        market_volatility,
        market_drift,
        dt,
        seed.wrapping_add(SEED_OFFSET_MARKET_PRICE),
    );

    Ok(MarketSetupBase {
        ctx,
        mint_a,
        mint_b,
        registry,
        vaults,
        market,
        initial_price,
        fee_rate: fee,
    })
}

pub fn build_market_environment(
    mode: EnvMode,
    vault_count: u8,
    deposit_a: u64,
    deposit_b: u64,
    market_volatility: f64,
    market_drift: f64,
    seed: u64,
    fee: FeeOption,
    payer_liquidity_multiplier: u64,
) -> Result<MarketSimulationEnvironment> {
    let base = setup_market_base(
        vault_count,
        deposit_a,
        deposit_b,
        market_volatility,
        market_drift,
        seed,
        fee,
        payer_liquidity_multiplier,
    )?;

    Ok(MarketSimulationEnvironment {
        ctx: base.ctx,
        mint_a: base.mint_a,
        mint_b: base.mint_b,
        registry: base.registry,
        vaults: base.vaults,
        market: base.market,
        initial_price: base.initial_price,
        fee_rate: base.fee_rate,
        mode,
    })
}

pub fn setup_market_environment_generic<B, F>(
    mode: EnvMode,
    vault_count: u8,
    deposit_a: u64,
    deposit_b: u64,
    market_volatility: f64,
    market_drift: f64,
    seed: u64,
    arbitrageur_count: u8,
    arbitrage_config: ArbitrageConfig,
    fee: FeeOption,
    balance_picker: F,
) -> Result<(Option<ArbitrageManager<B>>, MarketSimulationEnvironment)>
where
    B: ArbitrageBotFactory + ManagedArbitrageBot,
    B::SharedState: Default,
    F: Fn(u64, u64, u8) -> (u64, u64),
{
    // Validate vault count for dual mode
    if mode == EnvMode::Dual && vault_count < 2 {
        anyhow::bail!(
            "Dual market environment requires at least 2 vaults, got: {}",
            vault_count
        );
    }

    let mut market_env = build_market_environment(
        mode,
        vault_count,
        deposit_a,
        deposit_b,
        market_volatility,
        market_drift,
        seed,
        fee,
        vault_count as u64,
    )?;

    if arbitrageur_count == 0 {
        return Ok((None, market_env));
    }

    let mut arb_manager: ArbitrageManager<B> = new_generic_manager();

    for i in 0..arbitrageur_count {
        let (arb_balance_a, arb_balance_b) = balance_picker(deposit_a, deposit_b, vault_count);

        add_generic_arbitrageur::<B>(
            &mut arb_manager,
            &mut market_env.ctx,
            market_env.mint_a,
            market_env.mint_b,
            arb_balance_a,
            arb_balance_b,
            arbitrage_config.clone(),
            seed.wrapping_add(SEED_OFFSET_ARBITRAGEUR_BASE).wrapping_add(i as u64),
        )?;
    }

    Ok((Some(arb_manager), market_env))
}

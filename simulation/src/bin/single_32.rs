use anyhow::Result;
use simulation::scenarios::run_scenario_monte_carlo;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

const SCENARIO_INDEX: usize = 1; // scenario_2_single_with_arb
const USER_COUNT: usize = 32;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    run_scenario_monte_carlo(SCENARIO_INDEX, USER_COUNT)
}

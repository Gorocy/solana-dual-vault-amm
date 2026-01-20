// #[cfg(test)]
// mod tests {
//     use anyhow::Result;
//     use crate::utils::{
//         actors::arbitrageur_bot::{ArbitrageConfig, single::manager::{SingleArbitrageManager, new_single_manager}}, enviroment::single::setup::setup_market_environment
//     };

//     #[test]
//     fn test_arbitrage_bot_creation() -> Result<()> {
//         let (mut _arbitrageur_manager, env) = setup_market_environment(
//             1,
//             1_000_000,
//             1_000_000,
//             0.1,
//             0.0,
//             42,
//             crate::utils::ix::FeeOption::Tier30,
//         )?;

//         let mut ctx = env.ctx;
//         let mut bot = new_single_manager::new();

//         bot.add_arbitrageur(
//             &mut ctx,
//             env.mint_a,
//             env.mint_b,
//             500_000,
//             500_000,
//             ArbitrageConfig::default(),
//             123,
//         )
//         ?;

//         let all_bots = bot.get_bot_balances(&ctx.svm);
//         assert_eq!(all_bots[0].1, 500_000);
//         assert_eq!(all_bots[0].2, 500_000);
//         Ok(())
//     }

//     #[test]
//     fn test_arbitrage_manager() -> Result<()> {
//         let (mut _arbitrageur_manager, mut env) = setup_market_environment(
//             2,
//             1_000_000,
//             1_000_000,
//             0.1,
//             0.0,
//             42,
//             crate::utils::ix::FeeOption::Tier30,
//         )?;

//         let mut manager = SingleArbitrageManager::new();

//         // Dodaj kilku arbitrażystów
//         for i in 0..3 {
//             {
//                 manager
//                     .add_arbitrageur(
//                         &mut env.ctx,
//                         env.mint_a,
//                         env.mint_b,
//                         1_000_000,
//                         1_000_000,
//                         ArbitrageConfig::default(),
//                         100 + i,
//                     )
//                     ?;
//             }
//         }

//         assert_eq!(manager.bots.len(), 3);

//         let bot_balances = manager.get_bot_balances(&env.ctx.svm);
//         assert_eq!(bot_balances.len(), 3);

//         // Każdy bot powinien mieć balance
//         for (_pubkey, balance_a, balance_b) in bot_balances {
//             assert_eq!(balance_a, 1_000_000);
//             assert_eq!(balance_b, 1_000_000);
//         }
//         Ok(())
//     }
// }

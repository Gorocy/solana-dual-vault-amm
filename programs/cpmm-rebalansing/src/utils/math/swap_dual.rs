use anchor_lang::prelude::*;

// Assume these are defined in your lib.rs or accessible modules
use super::SafeMath;
use crate::{integer_sqrt, VaultAssets, FEE_DENOMINATOR};

/// Structure holding references to vault assets to perform virtual operations.
pub struct SwapUtilsVirtualVault;

impl SwapUtilsVirtualVault {


    /// Main point to calculate the entire dual swap.
    ///
    /// Returns:
    /// (
    ///     total_output,           // Total amount of Y token user receives
    ///     (input_v1, input_v2),   // How much X token goes into Vault1 vs Vault2
    ///     (output_v1, output_v2), // How much Y token comes out of Vault1 vs Vault2
    ///     (fee_v1, fee_v2)        // Fee split based on liquidity depth
    /// )
    pub fn calculate_routing_and_output(
        amount_in_raw: u64,
        fee_rate: u64,
        vault1: &VaultAssets,
        vault2: &VaultAssets,
    ) -> Result<(u64, (u64, u64), (u64, u64), (u64, u64))> {
        // 1. Calculate Liquidities (L) for both vaults
        let (l1, l2, l_total) = Self::calculate_liquidities(vault1, vault2)?;

        // 2. Calculate Fee and Net Input
        let (amount_in_net, fee_v1, fee_v2) =
            Self::calculate_fee_split(amount_in_raw, fee_rate, l1, l2, l_total)?;
//         // 3. Calculate Virtual Output (using combined reserves)
//         let total_output = Self::calculate_virtual_output(amount_in_net, vault1, vault2)?;

        // 3. Calculate Optimal Input Split (Rebalancing)
        let (in_v1, in_v2) = Self::calculate_optimal_input_split(amount_in_net, l1, l2, l_total, vault1, vault2)?;

        // 4. Calculate Independent Local Output for each vault
        let out_v1 = Self::calculate_local_output(in_v1, vault1)?;
        let out_v2 = Self::calculate_local_output(in_v2, vault2)?;

        // 5. Total output is the sum of safely calculated local outputs
        let total_output = out_v1.safe_add(out_v2)?;

        // 6. Validate physical liquidity constraints
        Self::validate_liquidity(out_v1, out_v2, vault1, vault2)?;

        Ok((
            total_output,
            (in_v1, in_v2),
            (out_v1, out_v2),
            (fee_v1, fee_v2),
        ))
    }

    /// Step 1: Calculate liquidity depth (L = sqrt(x * y)) for both vaults.
    fn calculate_liquidities(vault1: &VaultAssets, vault2: &VaultAssets) -> Result<(u128, u128, u128)> {
        // Helper closure to calculate sqrt
        let calculate_l = |incoming: u128, outgoing: u128| -> Result<u128> {
            let product = incoming.safe_mul(outgoing)?;
            let l = integer_sqrt(product);
            Ok(l as u128)
        };

        let l1 = calculate_l(vault1.incoming, vault1.outgoing)?;
        let l2 = calculate_l(vault2.incoming, vault2.outgoing)?;

        let l_total = l1.safe_add(l2)?;

        if l_total == 0 {
            return err!(SwapError::InsufficientLiquidity);
        }

        Ok((l1, l2, l_total))
    }

    /// Step 2: Calculate fees and distribute them based on Liquidity Depth.
    /// Note: Fees are distributed based on 'L', not on where the swap actually goes.
    fn calculate_fee_split(
        amount_in_raw: u64,
        fee_rate: u64,
        l1: u128,
        l2: u128,
        l_total: u128,
    ) -> Result<(u64, u64, u64)> {
        // Total Fee = (Input * Rate) / Denominator
        let fee_total = (amount_in_raw as u128)
            .safe_mul(fee_rate as u128)?
            .safe_div_ceil(FEE_DENOMINATOR as u128)? as u64;                    // fee_total = (amount_in_raw * fee_rate) / FEE_DENOMINATOR

        let amount_in_net = amount_in_raw.safe_sub(fee_total)?;

        // Fee Vault 1 = Total Fee * (L1 / L_Total)
        let fee_v1 = if l1 > l2 {                                               // Vault 1 is deeper, so it gets a larger share of the fee
            (fee_total as u128).safe_mul(l1)?.safe_div(l_total)? as u64         // fee_v1 = fee_total * ( Liq_v1 / Liq_Total ) .floor
        } else {
            (fee_total as u128).safe_mul(l1)?.safe_div_ceil(l_total)? as u64    // fee_v1 = fee_total * ( Liq_v1 / Liq_Total ) .ceil  <--  with ceiling to ensure we don't undercharge the deeper vault
        };                                    // with  ceiling  to ensure we don't undercharge the deeper vault
                                              // mi wydaje się, że ceiling powinien być na tym drugim (tym wcześniejszym == tym pierwszym)
        let fee_v2 = fee_total.safe_sub(fee_v1)?;                               // fee_v2 = fee_total - fee_v1

        Ok((amount_in_net, fee_v1, fee_v2))
    }
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
//     /// Step 3: Calculate output using the "Virtual Vault" concept.
//     /// We treat reserves as if they were pooled together: X_total and Y_total.
//     fn calculate_virtual_output(amount_in_net: u64, vault1: &VaultAssets, vault2: &VaultAssets) -> Result<u64> {
//         let virtual_x = (vault1.incoming).safe_add(vault2.incoming)?;
//         let virtual_y = (vault1.outgoing).safe_add(vault2.outgoing)?;
//
//         // CPMM Formula: dy = (y * dx) / (x + dx)
//         let numerator = virtual_y.safe_mul(amount_in_net as u128)?;
//         let denominator = virtual_x.safe_add(amount_in_net as u128)?;
//
//         let amount_out = numerator.safe_div(denominator)? as u64;
//
//         if amount_out == 0 {
//             return err!(SwapError::InsufficientOutputAmount);
//         }
//
//         Ok(amount_out)
//     }////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Step 3: Determine how to split the input to rebalance the vaults.
    /// Formula: delta_x1 = [L1 * (x2 + Xin) - L2 * x1] / (L1 + L2)
    fn calculate_optimal_input_split(
        amount_in_net: u64,
        l1: u128,
        l2: u128,
        l_total: u128,
        vault1: &VaultAssets,
        vault2: &VaultAssets,
    ) -> Result<(u64, u64)> {
        // x1, x2 are current incoming reserves
        let x1 = vault1.incoming;         // reserve_v1_incoming_token = x1
        let x2 = vault2.incoming;         // reserve_v2_incoming_token = x2
        let x_in = amount_in_net as u128; // input_netto_for_split  =  Xin  =?  total input we want to split

        // Calculate Term 1: L1 * (x2 + Xin)          // Pierwszy wyraz licznika (    L_1 · (  x_2  +  X_in  )   )
        let term1 = l1.safe_mul(x2.safe_add(x_in)?)?; // term1≣potentialGravityForce_v1 = l1≣Liquidity_v1.safe_mul(x2≣reserve_v2_incoming_token.safe_add(x_in≣input_netto_for_split)?)?;
                        // granicznie może się zdarzyć overflow, ale to by oznaczało, że mamy do czynienia z ogromną ilością płynności i wejściem, co jest mało prawdopodobne w praktyce
        // Calculate Term 2: L2 * x1                  // Drugi    wyraz licznika (    L_2 · x_1                  )
        let term2 = l2.safe_mul(x1)?;                 //      ≣potentialGravityForce_v2     ≣Liquidity_v2.safe_mul(x1≣reserve_v1_incoming_token)?;

        if term1 > term2 {
            // Vault 1 needs more X to reach equilibrium
            let numerator = term1.safe_sub(term2)?;
            let optimal_v1 = numerator.safe_div(l_total)? as u64;
                                            //  Liquidity_total  //  Całkowita płynność (L_1 + L_2) w mianowniku normalizuje wynik do proporcji
            // Jeśli optymalna ilość dla V1 jest większa lub równa całkowitej netto, to V1 absorbuje cały swap, a V2 nic nie robi.
            if optimal_v1 >= amount_in_net {
                // Vault 1 is so "expensive" (low X) that it absorbs the entire trade
                Ok((amount_in_net, 0))
            } else {
                // Split between V1 and V2
                let optimal_v2 = amount_in_net.safe_sub(optimal_v1)?;
                Ok((optimal_v1, optimal_v2))
            }
        } else {
            // If term1 <= term2, math says V1 should give back X.
            // Since we can only SWAP (add X), V1 gets 0, and V2 absorbs everything.
            // This happens when V2 has a much better price.
            Ok((0, amount_in_net))
        }
    }

    /// Step 4: Calculate safe output based on local reserves of a single vault
    /// using the standard CPMM formula: dy = (y * dx) / (x + dx)      // delta  Δy = (y * Δx) / (x + Δx)
    fn calculate_local_output(
        amount_in: u64,
         vault: &VaultAssets) -> Result<u64> {
        if amount_in == 0 { // wczesne przerwanie przy zerowym wejściu
            return Ok(0); // powrót z wartością zero dla skarbca nieotrzymującego wejścia
        }

        let numerator   = (vault.outgoing).safe_mul(amount_in as u128)?;    // licznik   (y * Δx) = reserve_outgoing_token * amount_in
        let denominator = (vault.incoming).safe_add(amount_in as u128)?;    // mianownik (x + Δx) = reserve_incoming_token + amount_in

        let amount_out = numerator.safe_div(denominator)? as u64;           // wynik   (Δy) = (y * Δx) / (x + Δx)

        if amount_out == 0 {
            return err!(SwapError::InsufficientLocalOutputAmount); // powrót z błędem za małego wyjścia z funkcji calculate_local_output
        }

        Ok(amount_out)
    }

    /// Step 6: Final safety check to ensure vaults have enough physical Y tokens.
    fn validate_liquidity(out_v1: u64, out_v2: u64, vault1: &VaultAssets, vault2: &VaultAssets) -> Result<()> {
        if out_v1 > vault1.outgoing as u64 || out_v2 > vault2.outgoing as u64 {
            return err!(SwapError::InsufficientLiquidity);
        }
        Ok(())
    }
}

#[error_code]
pub enum SwapError {
    #[msg("Insufficient output amount calculated locally")]
    InsufficientLocalOutputAmount,

    #[msg("Insufficient liquidity in vaults")]
    InsufficientLiquidity,
}

#[test]
fn test_local_vault_preserves_cpmm_invariant() {
    use crate::SwapUtilsVirtualVault;

    // ===== Arrange =====

    // Vaulty są BARDZO podobne (ceny ~ takie same)
    let vault1 = VaultAssets {
        incoming: 1_000u128,
        outgoing: 1_000u128,
    };

    let vault2 = VaultAssets {
        incoming: 1_000u128,
        outgoing: 999u128, // minimalna asymetria
    };

    let amount_in_raw: u64 = 300;      // użytkownik chce wymienić 300 tokenów X, co stanowi znaczną część płynności, więc będzie to dobry test na sprawdzenie zachowania K
    let fee_rate: u64 = 0;             // brak opłat, aby skupić się wyłącznie na zachowaniu CPMM i rebalansowaniu

    // ===== Act =====

    let (
        _total_out,       // całkowita ilość tokena Y, którą użytkownik otrzyma (nieistotna dla tego testu)
        ( in_v1,  in_v2), //  in token split  for each vault
        (out_v1, out_v2), // out token split  for each vault
        _fees,            // fee split (not relevant for this test)
    ) = SwapUtilsVirtualVault::calculate_routing_and_output(
        amount_in_raw,
        fee_rate,
        &vault1,
        &vault2,
    ).expect("calculation failed");

    // ===== Assert: INVARIANT CHECK =====

    // Vault 1
    let k1_before = vault1.incoming * vault1.outgoing;
    let k1_after =
        (vault1.incoming + in_v1 as u128) *
        (vault1.outgoing - out_v1 as u128);     // po swapie, nowe rezerwy w Vault 1 to (x1 + in_v1) i (y1 - out_v1), więc nowe K to (x1 + in_v1) * (y1 - out_v1)

    // Vault 2
    let k2_before = vault2.incoming * vault2.outgoing;
    let k2_after =
        (vault2.incoming + in_v2 as u128) *
        (vault2.outgoing - out_v2 as u128);     // po swapie, nowe rezerwy w Vault 2 to (x2 + in_v2) i (y2 - out_v2), więc nowe K to (x2 + in_v2) * (y2 - out_v2)

    println!("Vault1: k_before={}, k_after={}", k1_before, k1_after);
    println!("Vault2: k_before={}, k_after={}", k2_before, k2_after);
    println!("dx split: v1={}, v2={}", in_v1, in_v2);
    println!("dy split: v1={}, v2={}", out_v1, out_v2);

    // W zaktualizowanej logice wymuszamy zachowanie lub zwiększenie K
    // dla każdego skarbca indywidualnie.
    assert!(
        k1_after >= k1_before,
        "Invariant BROKEN for Vault 1"
    );

    assert!(
        k2_after >= k2_before,
        "Invariant BROKEN for Vault 2"
    );
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Original source and radius versions qualify every retained-root derivative denominator.

use super::{Fixture, ReciprocalDerivativeContract, TestResult, ledger, shared};
use crate::integration::lc_acceptance::{invariants, root_support as inputs};
use nrz_bindings::readings::{ReadingSource, ReadingSourceMap};
use nrz_matrix::{LcSourceBinding, SpectralError};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Native source composition retains its original deterministic plan and all six live channels.
/// ´test:integration:lc-source-retained-root-native-retirement´
#[test]
fn lc_source_retained_root_native_retirement() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_store_limits(["5", "1/10", "1/2000"], budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    retirement::<f32>(&fixture, &contract)?;
    retirement::<f64>(&fixture, &contract)
}

/// Each equal-bit source/total/error mutation makes both the old result and a new hit unavailable.
fn retirement<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
) -> TestResult {
    let context = ArithmeticContext::new()?;
    for change in 0..6 {
        let mut case = inputs::nominal::<P>(fixture, contract, &mut ledger()?)?;
        let binding = binding(fixture, &case)?;
        let original =
            case.heap
                .evaluate_lc_spectrum(binding.spectrum(), &context, &mut ledger()?)?;
        let result = case.heap.evaluate_lc_source_statistics_reusing(
            &binding,
            &original,
            &context,
            &mut ledger()?,
        )?;
        assert_eq!(result.spectrum().usage(), original.usage());
        assert!(result.covariance(&case.heap, &context).is_ok());
        invariants::mutate(&mut case.heap, &case.original, change)?;
        assert!(result.covariance(&case.heap, &context).is_err());
        assert!(result.sensitivities(&case.heap, &context).is_err());
        assert!(
            case.heap
                .evaluate_lc_source_statistics_reusing(
                    &binding,
                    &original,
                    &context,
                    &mut ledger()?,
                )
                .is_err()
        );
    }
    Ok(())
}

/// A symmetric retained radius may be too wide for a divisor even though the original positive box is valid.
/// ´test:integration:lc-source-retained-root-no-recompute-fallback´
#[test]
fn lc_source_retained_root_no_recompute_fallback() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_store_limits(["5", "1/10", "1/2000"], budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    separation::<f32>(&fixture, &contract)?;
    separation::<f64>(&fixture, &contract)
}

/// Independent positive dyadic inputs have a wide numerical-only image and a finite cold derivative.
fn separation<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
) -> TestResult {
    let quarter = P::ONE / P::from_u8(4);
    let sixteenth = P::ONE / P::from_u8(16);
    let spread = P::from_u8(15) / P::from_u8(16);
    let case = inputs::prepare(
        fixture,
        contract,
        [
            [quarter, quarter * spread, quarter * spread],
            [sixteenth, sixteenth * spread, sixteenth * spread],
        ],
        [
            P::ONE / P::from_u8(128),
            P::ONE / (P::from_u8(128) * P::from_u8(4)),
        ],
        true,
        &mut ledger()?,
    )?;
    let binding = binding(fixture, &case)?;
    let context = ArithmeticContext::new()?;
    let original = case
        .heap
        .evaluate_lc_spectrum(binding.spectrum(), &context, &mut ledger()?)?;
    let mut budget = ledger()?;
    assert!(matches!(
        case.heap
            .evaluate_lc_source_statistics_reusing(&binding, &original, &context, &mut budget),
        Err(SpectralError::NotSeparated)
    ));
    assert_eq!(budget.usage().work, 3);
    assert!(original.normalized(&case.heap, &context).is_ok());
    case.heap
        .evaluate_lc_source_statistics(&binding, &context, &mut ledger()?)?;
    Ok(())
}

/// Reordered original sources retain the ordinary native joint declaration and deterministic plan.
fn binding<P: Scalar + 'static>(
    fixture: &Fixture,
    case: &inputs::Case<P>,
) -> TestResult<LcSourceBinding<P>> {
    let sources = vec![case.original[1].clone(), case.original[0].clone()];
    let mut declared = Vec::new();
    for source in &sources {
        declared.push(ReadingSource::new(
            source.reading().clone(),
            &mut ledger()?,
        )?);
    }
    let map = ReadingSourceMap::new(
        &fixture.graph,
        declared,
        case.original.clone().map(|source| source.reading().clone()),
        &mut ledger()?,
    )?;
    let spectrum = fixture.plan(case.original.clone(), &mut ledger()?)?;
    Ok(LcSourceBinding::new(
        &case.heap,
        spectrum,
        case.derivative.clone(),
        map,
        sources,
        case.companion,
        &mut ledger()?,
    )?)
}

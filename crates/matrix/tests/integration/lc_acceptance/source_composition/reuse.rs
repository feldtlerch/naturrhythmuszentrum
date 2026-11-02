// SPDX-License-Identifier: AGPL-3.0-only
//! A retained native root supplies original-source cross blocks without a second root evaluation.

use super::{super::reciprocal_statistics as shared, mapped};
use crate::reading_testing::{TestResult, ledger};
use crate::spectral_testing::Fixture;
use nrz_matrix::{ReciprocalDerivativeContract, SpectralError};
use nrz_scalar::{ArithmeticContext, Scalar};

mod batch;
mod boundaries;
mod native;

/// Both formats preserve all signed source blocks, selected-copy identities and original receipts.
/// ´test:integration:lc-source-retained-root-cross-blocks´
#[test]
fn lc_source_retained_root_cross_blocks() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    for full in [false, true] {
        check::<f32>(&fixture, &contract, full)?;
        check::<f64>(&fixture, &contract, full)?;
    }
    Ok(())
}

/// The original result receipt and independent rational contraction survive reuse for each selection.
fn check<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
    full: bool,
) -> TestResult {
    let case = mapped::prepare::<P>(fixture, full, true)?;
    let context = ArithmeticContext::new()?;
    for indices in [[1, 5], [3, 7], [8, 9]] {
        let binding = mapped::binding(fixture, contract, &case, indices, &mut ledger()?)?;
        let mut budget = ledger()?;
        let original = case
            .heap
            .evaluate_lc_spectrum(binding.spectrum(), &context, &mut budget)?;
        let before = budget.usage();
        let result = case.heap.evaluate_lc_source_statistics_reusing(
            &binding,
            &original,
            &context,
            &mut budget,
        )?;
        assert_eq!(result.spectrum().usage(), original.usage());
        assert_eq!(result.usage()[0], before);
        assert_eq!(result.usage()[1], budget.usage());
        assert_eq!(result.binding().source_columns(), [1, 0]);
        assert_eq!(result.sensitivities(&case.heap, &context)?.len(), 2);
        mapped::oracle(result.covariance(&case.heap, &context)?, full)?;
    }
    Ok(())
}

/// Identical centers in another heap or independently prepared plan do not authenticate a retained root.
/// ´test:integration:lc-source-retained-root-owner-refusals´
#[test]
fn lc_source_retained_root_owner_refusals() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    owners::<f32>(&fixture, &contract)?;
    owners::<f64>(&fixture, &contract)
}

/// A refused lookup leaves both original live evaluations available to their own consumers.
fn owners<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
) -> TestResult {
    let case = mapped::prepare::<P>(fixture, false, true)?;
    let binding = mapped::binding(fixture, contract, &case, [1, 5], &mut ledger()?)?;
    let context = ArithmeticContext::new()?;
    let original = case
        .heap
        .evaluate_lc_spectrum(binding.spectrum(), &context, &mut ledger()?)?;
    let separate = mapped::binding(fixture, contract, &case, [1, 5], &mut ledger()?)?;
    let mut budget = ledger()?;
    assert!(matches!(
        case.heap.evaluate_lc_source_statistics_reusing(
            &separate,
            &original,
            &context,
            &mut budget
        ),
        Err(SpectralError::Identity)
    ));
    assert_eq!(budget.usage().work, 1);
    let foreign = mapped::prepare::<P>(fixture, false, true)?;
    let other = mapped::binding(fixture, contract, &foreign, [1, 5], &mut ledger()?)?;
    let other_root =
        foreign
            .heap
            .evaluate_lc_spectrum(other.spectrum(), &context, &mut ledger()?)?;
    assert!(
        case.heap
            .evaluate_lc_source_statistics_reusing(&binding, &other_root, &context, &mut ledger()?,)
            .is_err()
    );
    case.heap.evaluate_lc_source_statistics_reusing(
        &binding,
        &original,
        &context,
        &mut ledger()?,
    )?;
    assert!(other_root.normalized(&foreign.heap, &context).is_ok());
    Ok(())
}

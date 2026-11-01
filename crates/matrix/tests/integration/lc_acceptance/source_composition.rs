// SPDX-License-Identifier: AGPL-3.0-only
//! Source order is explicit and existing native LC partials supply every output block.

mod costs;
mod mapped;
mod refusals;

use super::{reciprocal_statistics as shared, root_support as inputs};
use crate::reading_testing::{TestResult, ledger};
use crate::reciprocal_testing::oracle::Rational;
use crate::spectral_testing::Fixture;
use nrz_bindings::readings::{ReadingSource, ReadingSourceMap};
use nrz_matrix::{LcSourceBinding, MomentAssurance, ReciprocalDerivativeContract};
use nrz_scalar::{ArithmeticContext, Scalar};

/// A permuted original source basis preserves both target maps and independently contracted signs.
/// ´test:integration:lc-source-composition-original-order-and-cross-blocks´
#[test]
fn lc_source_composition_original_order_and_cross_blocks() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_store_limits(["5", "1/10", "1/2000"], budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    check::<f32>(&fixture, &contract)?;
    check::<f64>(&fixture, &contract)
}

/// The native rank-one model supplies an independent rational eight-output oracle.
fn check<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
) -> TestResult {
    let mut case = inputs::nominal::<P>(fixture, contract, &mut ledger()?)?;
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
    let mut budget = ledger()?;
    let binding = LcSourceBinding::new(
        &case.heap,
        fixture.plan(case.original.clone(), &mut ledger()?)?,
        case.derivative.clone(),
        map,
        sources,
        case.companion,
        &mut budget,
    )?;
    assert_eq!(binding.source_columns(), [1, 0]);
    let context = ArithmeticContext::new()?;
    let mut evaluation = ledger()?;
    let result = case
        .heap
        .evaluate_lc_source_statistics(&binding, &context, &mut evaluation)?;
    assert_eq!(result.assurance(), MomentAssurance::LinearizedMoments);
    let covariance = result.covariance(&case.heap, &context)?;
    let gains = [
        (0, 1),
        (1, 320),
        (-1, 200_000),
        (0, 1),
        (0, 1),
        (-1, 4),
        (0, 1),
        (1, 4),
    ];
    for (row, &(left, ld)) in gains.iter().enumerate() {
        for (column, &(right, rd)) in gains.iter().enumerate() {
            let expected = Rational::new(left * right, ld * rd)?;
            assert!(expected.enclosed(
                covariance[row][column].lower().get().to_f64(),
                covariance[row][column].upper().get().to_f64()
            )?);
        }
    }
    assert_eq!(result.sensitivities(&case.heap, &context)?.len(), 2);
    let current = case.heap.reading_entries(&case.original[0])?;
    case.heap
        .replace_reading_entries(&case.original[0], &current, &mut ledger()?)?;
    assert!(result.covariance(&case.heap, &context).is_err());
    assert!(result.sensitivities(&case.heap, &context).is_err());
    assert!(
        case.heap
            .evaluate_lc_source_statistics(&binding, &context, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

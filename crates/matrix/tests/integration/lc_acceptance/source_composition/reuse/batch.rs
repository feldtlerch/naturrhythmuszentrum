// SPDX-License-Identifier: AGPL-3.0-only
//! Scheduled roots retain the original mapped source model and its complete signed cross blocks.

use super::{Fixture, ReciprocalDerivativeContract, TestResult, ledger, mapped, shared};
use nrz_bindings::readings::{ReadingSource, ReadingSourceMap};
use nrz_matrix::{LcSourceBinding, LcSpectralBatch, LcSpectralBatchLimits, LcSpectralSchema};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// The checked family producer supplies source composition without another root or source graph.
/// ´test:integration:lc-source-retained-scheduled-root-cross-blocks´
#[test]
fn lc_source_retained_scheduled_root_cross_blocks() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    let schema = LcSpectralSchema::new(
        &fixture.graph,
        fixture.law.clone(),
        LiteralLimits::default(),
        &mut budget,
    )?;
    for full in [false, true] {
        composed::<f32>(&fixture, &schema, &contract, full)?;
        composed::<f64>(&fixture, &schema, &contract, full)?;
    }
    Ok(())
}

fn composed<P: Scalar + 'static>(
    fixture: &Fixture,
    schema: &LcSpectralSchema,
    contract: &ReciprocalDerivativeContract,
    full: bool,
) -> TestResult {
    let context = ArithmeticContext::new()?;
    for indices in [[1, 5], [3, 7], [8, 9]] {
        let case = mapped::prepare::<P>(fixture, full, true)?;
        let targets = indices.map(|index| case.readings[index].clone());
        let spectrum = schema.bind(&case.heap, targets, &mut ledger()?)?;
        let binding = binding(fixture, contract, &case, spectrum.clone())?;
        let points = [
            (100, spectrum.clone()),
            (20, spectrum.clone()),
            (300, spectrum),
        ];
        let limits = LcSpectralBatchLimits {
            points: 3,
            bindings: 1,
            key_bytes: 4096,
        };
        let mut budget = ledger()?;
        let mut batch =
            LcSpectralBatch::prepare(schema, &case.heap, &points, limits, &context, &mut budget)?;
        batch.execute(&case.heap, &context, &mut budget)?;
        assert_eq!(batch.bindings(), 1);
        let original = batch.result(1, &case.heap, &context, &mut budget)?;
        let receipt = original.usage();
        let before = budget.usage().work;
        let statistics = case.heap.evaluate_lc_source_statistics_reusing(
            &binding,
            original,
            &context,
            &mut budget,
        )?;
        assert_eq!(budget.usage().work - before, if full { 754 } else { 742 });
        assert_eq!(statistics.spectrum().usage(), receipt);
        assert_eq!(
            batch.result(2, &case.heap, &context, &mut budget)?.usage(),
            receipt
        );
        mapped::oracle(statistics.covariance(&case.heap, &context)?, full)?;
        drop(batch);
        mapped::oracle(statistics.covariance(&case.heap, &context)?, full)?;
    }
    Ok(())
}

/// Prepare the original paired source map before any numerical node executes.
fn binding<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
    case: &mapped::Case<P>,
    spectrum: nrz_matrix::LcSpectrum<P>,
) -> TestResult<LcSourceBinding<P>> {
    let sources = vec![case.readings[4].clone(), case.readings[0].clone()];
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
        spectrum
            .inputs()
            .clone()
            .map(|source| source.reading().clone()),
        &mut ledger()?,
    )?;
    let derivative = case.heap.evaluate_reciprocal_derivative(
        contract,
        spectrum.inputs(),
        &ArithmeticContext::new()?,
        &mut ledger()?,
    )?;
    Ok(LcSourceBinding::new(
        &case.heap,
        spectrum,
        derivative,
        map,
        sources,
        case.companion,
        &mut ledger()?,
    )?)
}

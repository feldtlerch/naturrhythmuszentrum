// SPDX-License-Identifier: AGPL-3.0-only
//! A scheduled root feeds its complete predeclared heap bundle without another evaluation.

mod resources;

use super::{Case, Fixture, TestResult, case, ledger, prepare, schema};
use crate::reciprocal_testing::oracle::Rational;
use nrz_matrix::{LcSpectralBatch, LcSpectralPublication, LcSpectralSchema, PublishedLcSpectrum};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Scheduled rows and the heap bundle retain the identical original numerical producer receipt.
/// ´test:integration:lc-spectral-batch-publishes-retained-root-bundle´
#[test]
fn lc_spectral_batch_publishes_retained_root_bundle() -> TestResult {
    let fixture = Fixture::with_outputs(["5", "1/10", "1/2000"])?;
    publication::<f32>(&fixture)?;
    publication::<f64>(&fixture)
}

fn publication<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    let (mut case, batch, publication) = prepared::<P>(fixture, &schema, &context)?;
    let mut budget = ledger()?;
    let original = batch.result(2, &case.heap, &context, &mut budget)?.clone();
    let receipt = original.usage();
    let published =
        case.heap
            .publish_lc_spectrum_reusing(&publication, &original, &context, &mut budget)?;
    assert_eq!(published.evaluation().usage(), receipt);
    assert_eq!(
        batch.result(0, &case.heap, &context, &mut budget)?.usage(),
        receipt
    );
    assert_eq!(published.readings().bindings().len(), 6);
    for (binding, declared) in published
        .readings()
        .bindings()
        .iter()
        .zip(publication.coverage().readings())
    {
        assert_eq!(binding.reading(), declared.reading());
        case.heap.reading_entries(binding)?;
    }
    values(&published, &case.heap, &context)?;
    drop(batch);
    values(&published, &case.heap, &context)?;
    let output = &published.readings().bindings()[0];
    let radius = case.heap.reading_companion_entries(output, 0)?;
    case.heap
        .replace_reading_companion_entries(output, 0, &radius, &mut budget)?;
    for index in 0..6 {
        assert!(
            published
                .reading(index, &case.heap, &context, &mut budget)
                .is_err()
        );
    }
    original.normalized(&case.heap, &context)?;
    Ok(())
}

/// The complete result inventory is declared before the numerical stratum starts.
fn prepared<P: Scalar + 'static>(
    fixture: &Fixture,
    schema: &LcSpectralSchema,
    context: &ArithmeticContext,
) -> TestResult<(Case<P>, LcSpectralBatch<P>, LcSpectralPublication<P>)> {
    let case = case::<P>(fixture, schema, 0)?;
    let publication = fixture.publication(case.plans[0].clone(), &mut ledger()?)?;
    let mut batch = prepare(schema, &case, context, &mut ledger()?)?;
    batch.execute(&case.heap, context, &mut ledger()?)?;
    Ok((case, batch, publication))
}

/// Independent exact root/component values and each original own-target radius are all available.
fn values<P: Scalar + 'static>(
    published: &PublishedLcSpectrum<P>,
    heap: &nrz_matrix::Heap,
    context: &ArithmeticContext,
) -> TestResult {
    for (index, expected) in [(0, 8), (1, -8), (2, 0), (3, 0), (4, 8), (5, -8)] {
        let values = published.reading(index, heap, context, &mut ledger()?)?;
        let value = values[usize::from(index < 2)];
        assert!(Rational::new(expected, 1)?.covered(value.center.to_f64(), value.error.to_f64())?);
        assert!(value.error <= value.total);
        if index < 2 {
            assert!(values[0].center.same_bits(P::ZERO));
            assert!(values[0].total.same_bits(P::ZERO));
        }
    }
    Ok(())
}

/// Equality of inputs cannot substitute another prepared publication owner; refusal leaves no prefix.
/// ´test:integration:lc-spectral-retained-publication-owner-refusals´
#[test]
fn lc_spectral_retained_publication_owner_refusals() -> TestResult {
    let fixture = Fixture::with_outputs(["5", "1/10", "1/2000"])?;
    owners::<f32>(&fixture)?;
    owners::<f64>(&fixture)
}

fn owners<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    let (mut case, batch, publication) = prepared::<P>(fixture, &schema, &context)?;
    let original = batch.result(0, &case.heap, &context, &mut ledger()?)?;
    let different = fixture.publication(case.plans[1].clone(), &mut ledger()?)?;
    let before = case.heap.provenance_usage();
    assert!(matches!(
        case.heap
            .publish_lc_spectrum_reusing(&different, original, &context, &mut ledger()?),
        Err(nrz_matrix::SpectralError::Identity)
    ));
    assert_eq!(case.heap.provenance_usage(), before);
    let mut foreign = super::case::<P>(fixture, &schema, 0)?;
    assert!(
        foreign
            .heap
            .publish_lc_spectrum_reusing(&publication, original, &context, &mut ledger()?)
            .is_err()
    );
    let published =
        case.heap
            .publish_lc_spectrum_reusing(&publication, original, &context, &mut ledger()?)?;
    values(&published, &case.heap, &context)?;
    let before = case.heap.provenance_usage();
    assert!(
        case.heap
            .publish_lc_spectrum_reusing(&publication, original, &context, &mut ledger()?)
            .is_err()
    );
    assert_eq!(case.heap.provenance_usage(), before);
    Ok(())
}

/// Every original source/radius write prevents publication while leaving the output group absent.
/// ´test:integration:lc-spectral-retained-publication-stale-refusals´
#[test]
fn lc_spectral_retained_publication_stale_refusals() -> TestResult {
    let fixture = Fixture::with_outputs(["5", "1/10", "1/2000"])?;
    stale::<f32>(&fixture)?;
    stale::<f64>(&fixture)
}

fn stale<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    for channel in 0..6 {
        let (mut case, batch, publication) = prepared::<P>(fixture, &schema, &context)?;
        let original = batch
            .result(0, &case.heap, &context, &mut ledger()?)?
            .clone();
        super::super::invariants::mutate(&mut case.heap, &case.inputs[0], channel)?;
        let before = case.heap.provenance_usage();
        assert!(
            case.heap
                .publish_lc_spectrum_reusing(&publication, &original, &context, &mut ledger()?)
                .is_err()
        );
        assert_eq!(case.heap.provenance_usage(), before);
    }
    Ok(())
}

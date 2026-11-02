// SPDX-License-Identifier: AGPL-3.0-only
//! Literal retained-root publication limits preserve prior Work and unpublished output groups.

use super::{Fixture, TestResult, ledger, prepared, schema, values};
use nrz_bindings::{Budget, Limits, readings::algebra::Location};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Both formats pin the new consumer without changing the original cold publisher's limits.
/// ´test:integration:lc-spectral-retained-publication-literal-boundaries´
#[test]
fn lc_spectral_retained_publication_literal_boundaries() -> TestResult {
    let fixture = Fixture::with_outputs(["5", "1/10", "1/2000"])?;
    boundaries::<f32>(&fixture, [4_811, 1_010_772, 505_316])?;
    boundaries::<f64>(&fixture, [4_811, 1_010_888, 505_400])
}

/// Each independent allowance has exact, one-below and already-consumed Work twins.
fn boundaries<P: Scalar + 'static>(fixture: &Fixture, profile: [u64; 3]) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    for (kind, threshold) in profile.into_iter().enumerate() {
        for (available, prior, success) in [
            (threshold, 0, true),
            (threshold - 1, 0, false),
            (threshold, 1, kind != 0),
        ] {
            let (mut case, batch, publication) = prepared::<P>(fixture, &schema, &context)?;
            let original = batch.result(0, &case.heap, &context, &mut ledger()?)?;
            let mut budget = limited(kind, available)?;
            budget.work(prior, Location::new("prior retained publication"))?;
            let before = case.heap.provenance_usage();
            let result = case.heap.publish_lc_spectrum_reusing(
                &publication,
                original,
                &context,
                &mut budget,
            );
            assert_eq!(
                result.is_ok(),
                success,
                "format {:?}, kind {kind}, limit {available}, prior {prior}: {result:?}",
                P::PRECISION
            );
            assert!(budget.usage().work >= prior);
            let published = if let Ok(published) = result {
                assert_eq!(budget.usage().work, profile[0] + prior);
                published
            } else {
                assert_eq!(case.heap.provenance_usage(), before);
                case.heap.publish_lc_spectrum_reusing(
                    &publication,
                    original,
                    &context,
                    &mut ledger()?,
                )?
            };
            assert_eq!(published.evaluation().usage(), original.usage());
            values(&published, &case.heap, &context)?;
            for row in 0..3 {
                batch
                    .result(row, &case.heap, &context, &mut ledger()?)?
                    .normalized(&case.heap, &context)?;
            }
        }
    }
    prefix::<P>(fixture, profile)
}

/// Releasing a resident byte permits same-ledger publication without refunding refused Work.
fn prefix<P: Scalar + 'static>(fixture: &Fixture, profile: [u64; 3]) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    let (mut case, batch, publication) = prepared::<P>(fixture, &schema, &context)?;
    let original = batch.result(0, &case.heap, &context, &mut ledger()?)?;
    let mut budget = limited(1, profile[1])?;
    let before = case.heap.provenance_usage();
    let refused = budget.with_retained_payload(
        1,
        Location::new("resident retained publication prefix"),
        |budget| {
            case.heap
                .publish_lc_spectrum_reusing(&publication, original, &context, budget)
        },
    );
    assert!(refused.is_err());
    assert_eq!(case.heap.provenance_usage(), before);
    original.normalized(&case.heap, &context)?;
    let spent = budget.usage().work;
    assert!(spent > 0);
    let published =
        case.heap
            .publish_lc_spectrum_reusing(&publication, original, &context, &mut budget)?;
    assert_eq!(budget.usage().work, spent + profile[0]);
    assert_eq!(published.evaluation().usage(), original.usage());
    values(&published, &case.heap, &context)
}

/// Lower one independent dimension within the unchanged default envelope.
fn limited(kind: usize, available: u64) -> TestResult<Budget> {
    let mut limits = Limits::default();
    match kind {
        0 => limits.work = available,
        1 => limits.staged_payload_bytes = available,
        _ => limits.output_bytes = available,
    }
    Ok(Budget::new(limits)?)
}

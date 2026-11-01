// SPDX-License-Identifier: AGPL-3.0-only
//! Publication boundaries are measured independently of declaration, root and coefficient preparation.

use super::{Prepared, TestResult, inputs, prepared, publication, shared};
use nrz_bindings::{Budget, readings::algebra::Location};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Literal publication-only ceilings preserve admitted Work and original stamps on atomic refusal.
/// ´test:integration:lc-frequency-publication-boundaries´
#[test]
fn lc_frequency_publication_boundaries() -> TestResult {
    let prepared = prepared()?;
    boundaries::<f32>(&prepared, [918_744, 460_736])?;
    boundaries::<f64>(&prepared, [918_976, 460_936])
}

fn boundaries<P: Scalar + 'static>(prepared: &Prepared, bytes: [u64; 2]) -> TestResult {
    for (kind, required, prefix) in [(0, 3_844, 3_843), (1, bytes[0], 270), (2, bytes[1], 72)] {
        assert_eq!(attempt::<P>(prepared, kind, required, 0)?, (true, 3_844));
        assert_eq!(
            attempt::<P>(prepared, kind, required - 1, 0)?,
            (false, prefix)
        );
    }
    assert_eq!(attempt::<P>(prepared, 0, 3_844, 1)?, (false, 3_844));
    Ok(())
}

/// Every refusal validates original stamps and provenance and retries exactly the same output symbols.
fn attempt<P: Scalar + 'static>(
    prepared: &Prepared,
    kind: u8,
    available: u64,
    prior: u64,
) -> TestResult<(bool, u64)> {
    let mut setup = shared::policy()?;
    let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut setup)?;
    let plan = publication(prepared, &case, &mut setup)?;
    let before = case.heap.provenance_usage();
    let stamps = case
        .selected
        .iter()
        .map(|value| {
            case.heap
                .value_by_symbol(value.symbol(), value)
                .map(|view| view.stamp())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let values = case
        .original
        .iter()
        .map(|value| case.heap.reading_entries(value))
        .collect::<Result<Vec<_>, _>>()?;
    let mut limits = shared::policy()?.limits().clone();
    match kind {
        0 => limits.work = available,
        1 => limits.staged_payload_bytes = available,
        2 => limits.output_bytes = available,
        _ => return Err("publication boundary resource".into()),
    }
    let mut budget = Budget::new(limits)?;
    budget.work(
        prior,
        Location::new("prior connected frequency publication"),
    )?;
    let context = ArithmeticContext::new()?;
    let result = case.heap.publish_lc_frequency(&plan, &context, &mut budget);
    let success = result.is_ok();
    if success {
        result?.covariance(&case.heap, &context)?;
    } else {
        assert_eq!(case.heap.provenance_usage(), before);
        for ((binding, expected), reading) in case.selected.iter().zip(stamps).zip(&case.original) {
            assert_eq!(
                case.heap
                    .value_by_symbol(binding.symbol(), binding)?
                    .stamp(),
                expected
            );
            case.heap.reading_entries(reading)?;
        }
        for (reading, value) in case.original.iter().zip(values) {
            assert_eq!(case.heap.reading_entries(reading)?, value);
        }
        let retry = case
            .heap
            .publish_lc_frequency(&plan, &context, &mut shared::policy()?)?;
        assert_eq!(retry.readings().bindings().len(), 5);
        assert_eq!(retry.readings().bindings()[0].companions().len(), 7);
    }
    Ok((success, budget.usage().work))
}

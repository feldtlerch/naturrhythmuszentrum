// SPDX-License-Identifier: AGPL-3.0-only
//! Literal scheduler boundaries exclude already admitted sources, schema and binding preparation.

use super::{Case, Fixture, TestResult, case, ledger, prepare, schema};
use nrz_bindings::{Budget, Limits, readings::algebra::Location};
use nrz_matrix::{LcSpectralBatch, LcSpectralSchema, LcSpectralStatus as Status};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Both formats pin the new closed owner without changing any existing child or default limit.
/// ´test:integration:lc-spectral-batch-literal-boundaries´
#[test]
fn lc_spectral_batch_literal_boundaries() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    boundaries::<f32>(&fixture, [494, 5656, 3160, 4816])?;
    boundaries::<f64>(&fixture, [494, 5992, 3272, 4928])
}

fn boundaries<P: Scalar + 'static>(fixture: &Fixture, profile: [u64; 4]) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    let case = case::<P>(fixture, &schema, 0)?;
    let mut budget = ledger()?;
    let batch = schedule(&schema, &case, &context, &mut budget)?;
    assert_eq!(budget.usage().work, 494);
    assert_eq!(batch.inventory().key_bytes, 88);
    assert_eq!(schema.retained_payload_bytes(), 152);
    assert_eq!(case.plans[0].retained_payload_bytes(), 752);
    assert_eq!(batch.inventory().logical_payload_bytes, profile[2]);
    assert_eq!(batch.inventory().observed_capacity_bytes, profile[2]);
    let coefficients = batch.coefficient_usage().ok_or("coefficient receipt")?;
    assert_eq!(coefficients[1].work - coefficients[0].work, 244);
    for row in [0, 3] {
        let root = batch
            .result(row, &case.heap, &context, &mut budget)?
            .usage();
        assert_eq!(root[1].work - root[0].work, 115);
    }
    allowance_twins(&schema, &case, &context, &batch, profile)?;
    prefix(&schema, &case, &context, profile[1])
}

/// Each independent allowance has an exact, one-below and already-consumed twin.
fn allowance_twins<P: Scalar + 'static>(
    schema: &LcSpectralSchema,
    case: &Case<P>,
    context: &ArithmeticContext,
    batch: &LcSpectralBatch<P>,
    profile: [u64; 4],
) -> TestResult {
    for (kind, threshold) in profile[..3].iter().copied().enumerate() {
        for (available, prior, expected) in [
            (threshold, 0, true),
            (threshold - 1, 0, false),
            (threshold, 1, kind != 0),
        ] {
            let mut budget = limited(kind, available)?;
            budget.work(prior, Location::new("prior scheduler work"))?;
            let result = schedule(schema, case, context, &mut budget);
            let success = result
                .as_ref()
                .is_ok_and(|batch| batch.status(3) == Some(Status::Successful));
            assert_eq!(
                success,
                expected,
                "format {:?}, kind {kind}, limit {available}, prior {prior}",
                P::PRECISION
            );
            assert!(budget.usage().work >= prior);
            if success {
                assert_eq!(budget.usage().work, 494 + prior);
            }
        }
    }
    for (work, prior, expected) in [(3, 0, true), (2, 0, false), (3, 1, false)] {
        let mut budget = limited(0, work)?;
        budget.work(prior, Location::new("prior result lookup"))?;
        assert_eq!(
            batch.result(0, &case.heap, context, &mut budget).is_ok(),
            expected
        );
    }
    for (staged, expected) in [(profile[3], true), (profile[3] - 1, false)] {
        assert_eq!(
            batch
                .result(0, &case.heap, context, &mut limited(1, staged)?)
                .is_ok(),
            expected
        );
    }
    Ok(())
}

/// A released resident prefix permits a new finite owner without recovering spent Work.
fn prefix<P: Scalar + 'static>(
    schema: &LcSpectralSchema,
    case: &Case<P>,
    context: &ArithmeticContext,
    staged: u64,
) -> TestResult {
    let mut budget = limited(1, staged)?;
    let mut refused =
        budget.with_retained_payload(1, Location::new("resident scheduler prefix"), |budget| {
            schedule(schema, case, context, budget)
        })?;
    assert_ne!(refused.status(3), Some(Status::Successful));
    let spent = budget.usage().work;
    assert!(spent > 0);
    assert!(refused.execute(&case.heap, context, &mut budget).is_err());
    let fresh = schedule(schema, case, context, &mut budget)?;
    assert_eq!(fresh.status(3), Some(Status::Successful));
    assert_eq!(budget.usage().work, spent + 494);
    Ok(())
}

/// A late common Work refusal does not erase the earlier root or any of its original row aliases.
/// ´test:integration:lc-spectral-batch-late-resource-refusal´
#[test]
fn lc_spectral_batch_late_resource_refusal() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    late::<f32>(&fixture)?;
    late::<f64>(&fixture)
}

fn late<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    let case = case::<P>(fixture, &schema, 0)?;
    let mut budget = limited(0, 493)?;
    let mut batch = schedule(&schema, &case, &context, &mut budget)?;
    assert_eq!(batch.coefficient_status(), Status::Successful);
    for row in 0..3 {
        assert_eq!(batch.status(row), Some(Status::Successful));
        batch.result(row, &case.heap, &context, &mut ledger()?)?;
    }
    assert_eq!(batch.status(3), Some(Status::ResourceRefused));
    assert!(batch.failure(3).is_some());
    let spent = budget.usage().work;
    assert!(batch.execute(&case.heap, &context, &mut budget).is_err());
    assert_eq!(budget.usage().work, spent);
    Ok(())
}

fn schedule<P: Scalar + 'static>(
    schema: &LcSpectralSchema,
    case: &Case<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> TestResult<LcSpectralBatch<P>> {
    let mut batch = prepare(schema, case, context, budget)?;
    batch.execute(&case.heap, context, budget)?;
    Ok(batch)
}

fn limited(kind: usize, available: u64) -> TestResult<Budget> {
    let mut limits = Limits::default();
    match kind {
        0 => limits.work = available,
        1 => limits.staged_payload_bytes = available,
        _ => limits.output_bytes = available,
    }
    Ok(Budget::new(limits)?)
}

/// Measure complete scheduler and result-lookup boundaries after owned fault layout changes.
/// ´test:integration:lc-spectral-batch-owned-fault-measurement´
#[test]
#[ignore = "development measurement of owned fault resource boundaries"]
fn lc_spectral_batch_owned_fault_measurement() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    measure_owned::<f32>(&fixture)?;
    measure_owned::<f64>(&fixture)
}

#[expect(
    clippy::print_stdout,
    reason = "Explicit resource measurement emits its finite profile"
)]
fn measure_owned<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    let case = case::<P>(fixture, &schema, 0)?;
    let batch = schedule(&schema, &case, &context, &mut ledger()?)?;
    let mut profile = [0; 4];
    for (kind, value) in profile.iter_mut().enumerate() {
        let mut low = 0;
        let mut high = 20_000;
        while low < high {
            let middle = low + (high - low) / 2;
            let success = if kind == 3 {
                batch
                    .result(0, &case.heap, &context, &mut limited(1, middle)?)
                    .is_ok()
            } else {
                schedule(&schema, &case, &context, &mut limited(kind, middle)?)
                    .is_ok_and(|batch| batch.status(3) == Some(Status::Successful))
            };
            if success {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        *value = low;
    }
    println!("OWNED-FAULT-SPECTRAL {:?} {profile:?}", P::PRECISION);
    boundaries::<P>(fixture, profile)
}

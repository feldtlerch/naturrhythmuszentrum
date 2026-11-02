// SPDX-License-Identifier: AGPL-3.0-only
//! Finite admission and one cumulative attempt have no implicit cache eviction or retry.

use super::{Fixture, TestResult, case, ceilings, ledger, points, prepare, schema};
use nrz_bindings::{Budget, Limits, readings::algebra::Location};
use nrz_matrix::{LcSpectralBatch, LcSpectralStatus as Status};
use nrz_scalar::ArithmeticContext;

/// Admission bounds every row and retained key before numerical execution and refuses foreign authority.
/// ´test:integration:lc-spectral-batch-bounded-admission´
#[test]
fn lc_spectral_batch_bounded_admission() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    let schema = schema(&fixture)?;
    let context = ArithmeticContext::new()?;
    let case = case::<f64>(&fixture, &schema, 0)?;
    let batch = prepare(&schema, &case, &context, &mut ledger()?)?;
    for limits in [
        nrz_matrix::LcSpectralBatchLimits {
            points: 3,
            ..ceilings()
        },
        nrz_matrix::LcSpectralBatchLimits {
            bindings: 1,
            ..ceilings()
        },
        nrz_matrix::LcSpectralBatchLimits {
            key_bytes: batch.inventory().key_bytes - 1,
            ..ceilings()
        },
    ] {
        assert!(
            LcSpectralBatch::prepare(
                &schema,
                &case.heap,
                &points(&case),
                limits,
                &context,
                &mut ledger()?
            )
            .is_err()
        );
    }
    let mut duplicate = points(&case);
    duplicate[3].0 = duplicate[0].0;
    assert!(
        LcSpectralBatch::prepare(
            &schema,
            &case.heap,
            &duplicate,
            ceilings(),
            &context,
            &mut ledger()?
        )
        .is_err()
    );
    let foreign = super::schema(&fixture)?;
    assert!(
        LcSpectralBatch::prepare(
            &foreign,
            &case.heap,
            &points(&case),
            ceilings(),
            &context,
            &mut ledger()?
        )
        .is_err()
    );
    let (other, _) = fixture.import([[0.25_f64, 0.0, 0.0], [0.0625, 0.0, 0.0]])?;
    assert!(
        LcSpectralBatch::prepare(
            &schema,
            &other,
            &points(&case),
            ceilings(),
            &context,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Even a zero-Work preflight refusal consumes the declared attempt and leaves the complete suffix.
/// ´test:integration:lc-spectral-batch-cumulative-attempt´
#[test]
fn lc_spectral_batch_cumulative_attempt() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    let schema = schema(&fixture)?;
    let context = ArithmeticContext::new()?;
    let case = case::<f64>(&fixture, &schema, 0)?;
    let mut batch = prepare(&schema, &case, &context, &mut ledger()?)?;
    let mut budget = Budget::new(Limits {
        work: 1,
        ..Limits::default()
    })?;
    budget.work(1, Location::new("prior caller work"))?;
    batch.execute(&case.heap, &context, &mut budget)?;
    assert_eq!(budget.usage().work, 1);
    assert_eq!(batch.coefficient_status(), Status::ResourceRefused);
    for row in 0..3 {
        assert_eq!(batch.status(row), Some(Status::Blocked));
    }
    assert_eq!(batch.status(3), Some(Status::Blocked));
    assert!(batch.failure(3).is_some());
    assert!(batch.execute(&case.heap, &context, &mut ledger()?).is_err());
    Ok(())
}

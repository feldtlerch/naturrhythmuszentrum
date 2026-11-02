// SPDX-License-Identifier: AGPL-3.0-only
//! Full original input closures govern reuse independently of point order and equal centers.

use super::{Fixture, TestResult, case, ledger, prepare, schema};
use nrz_matrix::{LcSpectralBatch, LcSpectralStatus as Status};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Every original center and radius retires all aliases, retaining the original coarse fallback.
/// ´test:integration:lc-spectral-batch-revalidates-all-channels´
#[test]
fn lc_spectral_batch_revalidates_all_channels() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    channels::<f32>(&fixture)?;
    channels::<f64>(&fixture)
}

fn channels<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    for change in 0..6 {
        let mut case = case::<P>(fixture, &schema, 0)?;
        let mut batch = prepare(&schema, &case, &context, &mut ledger()?)?;
        batch.execute(&case.heap, &context, &mut ledger()?)?;
        super::super::invariants::mutate(&mut case.heap, &case.inputs[0], change)?;
        for row in 0..3 {
            assert!(
                batch
                    .result(row, &case.heap, &context, &mut ledger()?)
                    .is_err()
            );
            assert_eq!(batch.status(row), Some(Status::Successful));
        }
        assert_eq!(
            batch
                .result(3, &case.heap, &context, &mut ledger()?)
                .is_ok(),
            change >= 2,
            "channel {change}: original value writes retain the WholeBase graph fallback"
        );
        assert_eq!(batch.coefficient_status(), Status::Successful);
        assert!(prepare(&schema, &case, &context, &mut ledger()?).is_err());
        assert_eq!(batch.bindings(), 2);
    }
    Ok(())
}

/// Coarse retirement remains conservative; a later domain refusal preserves earlier success.
/// ´test:integration:lc-spectral-batch-retains-independent-outcomes´
#[test]
fn lc_spectral_batch_retains_independent_outcomes() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    independent::<f32>(&fixture)?;
    independent::<f64>(&fixture)
}

fn independent<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    let schema = schema(fixture)?;
    let context = ArithmeticContext::new()?;
    let mut case = case::<P>(fixture, &schema, 0)?;
    let mut batch = prepare(&schema, &case, &context, &mut ledger()?)?;
    super::super::invariants::mutate(&mut case.heap, &case.inputs[0], 0)?;
    batch.execute(&case.heap, &context, &mut ledger()?)?;
    for row in 0..4 {
        assert_eq!(batch.status(row), Some(Status::BindingRefused));
        assert!(batch.failure(row).is_some());
        assert!(
            batch
                .result(row, &case.heap, &context, &mut ledger()?)
                .is_err()
        );
    }
    let mut case = super::case::<P>(fixture, &schema, 0)?;
    let invalid = fixture.import_into(
        &mut case.heap,
        [[P::ONE, P::ZERO, P::ZERO], [P::ZERO, P::ZERO, P::ZERO]],
        &mut ledger()?,
    )?;
    let invalid = schema.bind(&case.heap, invalid, &mut ledger()?)?;
    let points = [(7, case.plans[2].clone()), (2, invalid)];
    let mut batch = LcSpectralBatch::prepare(
        &schema,
        &case.heap,
        &points,
        super::ceilings(),
        &context,
        &mut ledger()?,
    )?;
    batch.execute(&case.heap, &context, &mut ledger()?)?;
    assert_eq!(batch.status(0), Some(Status::Successful));
    assert_eq!(batch.status(1), Some(Status::DomainUnavailable));
    assert!(
        batch
            .result(1, &case.heap, &context, &mut ledger()?)
            .is_err()
    );
    batch.result(0, &case.heap, &context, &mut ledger()?)?;
    Ok(())
}

/// Equal numerical inputs in separately imported physical instances remain different invariant keys.
/// ´test:integration:lc-spectral-batch-keeps-original-source-identity´
#[test]
fn lc_spectral_batch_keeps_original_source_identity() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    let schema = schema(&fixture)?;
    let context = ArithmeticContext::new()?;
    let mut case = case::<f64>(&fixture, &schema, 0)?;
    let inputs = fixture.import_into(
        &mut case.heap,
        [[0.25, 0.0, 0.0], [0.0625, 0.0, 0.0]],
        &mut ledger()?,
    )?;
    let equal = schema.bind(&case.heap, inputs, &mut ledger()?)?;
    let points = [(11, case.plans[0].clone()), (12, equal)];
    let mut batch = LcSpectralBatch::prepare(
        &schema,
        &case.heap,
        &points,
        super::ceilings(),
        &context,
        &mut ledger()?,
    )?;
    assert_eq!(batch.bindings(), 2);
    batch.execute(&case.heap, &context, &mut ledger()?)?;
    assert!(!std::ptr::eq(
        batch.result(0, &case.heap, &context, &mut ledger()?)?,
        batch.result(1, &case.heap, &context, &mut ledger()?)?
    ));
    Ok(())
}

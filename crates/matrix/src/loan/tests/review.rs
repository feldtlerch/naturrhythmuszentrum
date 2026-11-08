// SPDX-License-Identifier: AGPL-3.0-only
//! Formatting and escaped ordinary payloads preserve the numerical ownership boundary.

use super::{TestResult, fixture, granted};
use crate::{
    Binding, ExecutionAccount, ExecutionLimits, Extent, HandbackOutcome, Heap, HeapLimits,
    HostChannel, LoanError, Residence,
};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_scalar::Scalar;

/// A retained NaN hole cannot be observed through any enclosing formatter at exhausted Work.
/// ´test:unit:loan-formatting-never-loads-cleared-holes´
#[test]
fn loan_formatting_never_loads_cleared_holes() -> TestResult {
    let (mut heap, mut account, declaration) =
        fixture::<f64>(1, 1, [false, false], Limits::default())?;
    let mut loan = granted(heap.request_loan(declaration, &mut account))?;
    loan.write_component(HostChannel::Center, 0, 0, f64::NAN, &mut account)?;
    loan.clear_cell(HostChannel::Center, 0, &mut account)?;
    assert!(!format!("{loan:?}").contains("NaN"));
    let tile = match heap.handback(loan, &mut account) {
        HandbackOutcome::Published(tile, _) => tile,
        other => return Err(format!("{other:?}").into()),
    };
    let reader = tile.reader(&heap, &mut account)?;
    let remaining = account.limits().work - account.usage().work;
    account.ledger.work(remaining, crate::loan::AT)?;
    crate::loan::fault::reset_reads();
    assert!(matches!(
        reader.read_component(HostChannel::Center, 0, 0, &mut account),
        Err(LoanError::CoverageMismatch)
    ));
    let used = account.usage().work;
    for output in [
        format!("{tile:?}"),
        format!("{:?}", tile.pin()),
        format!("{reader:?}"),
        format!("{heap:?}"),
        format!("{:?}", heap.loans),
    ] {
        assert!(!output.contains("NaN"), "{output}");
    }
    assert_eq!(crate::loan::fault::reads(), 0);
    assert_eq!(account.usage().work, used);
    Ok(())
}

/// Ordinary copied floats retain residence after both the binding and source are gone.
/// ´test:unit:loan-residence-retains-escaped-copy-candidate´
#[test]
fn loan_residence_retains_escaped_copy_candidate() -> TestResult {
    escaped_copy::<f32>()?;
    escaped_copy::<f64>()
}
/// Both scalar widths debit bytes without inventing destination elements.
fn escaped_copy<P: Scalar + 'static>() -> TestResult {
    let mut heap = Heap::new(HeapLimits::default())?;
    let source = heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?;
    let bound = Binding::new(
        crate::binding_testing::operation(1, 1)?,
        vec![source.value()],
        ExecutionLimits::default(),
    )
    .validate(&mut heap)?;
    let candidate = bound.evaluate(bound.normalized_inputs()?)?;
    drop(bound);
    heap.retire(source.value())?;
    let mut account = ExecutionAccount::new(Budget::new(Limits::default())?)?;
    assert!(matches!(
        heap.residence(&mut account, 1000)?,
        Residence::Pending {
            elements: Some([0, 0]),
            ..
        }
    ));
    let live = heap.resource_bytes()?;
    drop(candidate);
    // One scalar backing and the exact one-element Payload backing survive the binding.
    assert_eq!(live - heap.resource_bytes()?, size_of::<P>() + 280);
    assert!(matches!(
        heap.residence(&mut account, 1000)?,
        Residence::Empty { .. }
    ));
    Ok(())
}

/// Contribution vectors remain covered when only their common center owner survives.
/// ´test:unit:loan-residence-retains-escaped-contribution-vectors´
#[test]
fn loan_residence_retains_escaped_contribution_vectors() -> TestResult {
    escaped_contributions::<f32>()?;
    escaped_contributions::<f64>()
}
/// Copy a published product so the ordinary route carries all three contribution owners.
fn escaped_contributions<P: Scalar + 'static>() -> TestResult {
    let mut heap = Heap::new(HeapLimits::default())?;
    let declarations = crate::kernel_testing::owner()?;
    let coordinate = crate::kernel_testing::axis(1)?;
    let shape = crate::kernel_testing::shape(&declarations, &coordinate, &coordinate)?;
    let source = heap.import_declared_bounds(
        Extent::new(1, 1)?,
        &Descriptor::ratio(),
        &[P::ONE],
        &[P::ONE],
    )?;
    let operation =
        crate::OperationDraft::matmul(shape.clone(), shape, crate::ShapeLimits::default())?
            .check(ExecutionLimits::default())?;
    let copied = crate::OperationDraft::copy(vec![operation.outputs()[0].shape().clone()])
        .check(ExecutionLimits::default())?;
    let bound = Binding::new(
        operation,
        vec![source.value(); 2],
        ExecutionLimits::default(),
    )
    .validate(&mut heap)?;
    let candidate = bound.evaluate(bound.normalized_inputs()?)?;
    let product = bound.publish(candidate)?;
    let output = product.output().first().ok_or("product")?.value();
    let bound =
        Binding::new(copied, vec![output], ExecutionLimits::default()).validate(&mut heap)?;
    let mut candidate = bound.evaluate(bound.normalized_inputs()?)?;
    drop(bound);
    heap.retire(source.value())?;
    heap.retire(output)?;
    heap.retire_execution(product.record())?;
    let payload = candidate.outputs.first_mut().ok_or("copy")?;
    assert!(payload.contributions.is_some());
    drop(payload.bounds.take());
    drop(payload.evaluation.take());
    let mut account = ExecutionAccount::new(Budget::new(Limits::default())?)?;
    assert!(matches!(
        heap.residence(&mut account, 1000)?,
        Residence::Pending {
            elements: Some([0, 0]),
            ..
        }
    ));
    let live = heap.resource_bytes()?;
    drop(candidate);
    // Center plus three contribution scalars share the 280-byte outer Payload owner.
    assert_eq!(live - heap.resource_bytes()?, 4 * size_of::<P>() + 280);
    assert!(matches!(
        heap.residence(&mut account, 1000)?,
        Residence::Empty { .. }
    ));
    Ok(())
}

/// Successful and refused publication both destroy the unique outer-owner debit.
/// ´test:unit:loan-candidate-owner-debits-close-on-publication-and-refusal´
#[test]
fn loan_candidate_owner_debits_close_on_publication_and_refusal() -> TestResult {
    candidate_closure::<f32>()?;
    candidate_closure::<f64>()
}

/// The weak counter retains only its original metadata after either consuming outcome.
fn candidate_closure<P: Scalar + 'static>() -> TestResult {
    for refuse in [false, true] {
        let mut heap = Heap::new(HeapLimits::default())?;
        let source =
            heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?;
        let bound = Binding::new(
            crate::binding_testing::operation(1, 1)?,
            vec![source.value()],
            ExecutionLimits::default(),
        )
        .validate(&mut heap)?;
        let trace = bound.resources.trace.downgrade();
        let mut candidate = bound.evaluate(bound.normalized_inputs()?)?;
        if refuse {
            candidate.revision ^= 1;
            let failure = bound
                .publish(candidate)
                .err()
                .ok_or("altered candidate published")?;
            assert!(matches!(
                failure.cause(),
                crate::OperationError::CandidateContract
            ));
        } else {
            drop(bound.publish(candidate)?);
        }
        assert_eq!(
            trace.owned_bytes()?,
            crate::execution::Trace::metadata_bytes()
        );
    }
    Ok(())
}

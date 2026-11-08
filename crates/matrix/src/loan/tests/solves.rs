// SPDX-License-Identifier: AGPL-3.0-only
//! Solve account boundaries include concrete arrays beside the preserved backend allowance.

use super::TestResult;
use crate::{
    Binding, ExecutionAccount, ExecutionLimits, Extent, Heap, HeapError, OperationDraft,
    OperationError, ShapeLimits,
};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_scalar::Scalar;

/// Plain solves and inverses share the numerical schedule; restoration and recovery add fixed stages.
fn fixture<P: Scalar + 'static>(
    inverse: bool,
    restored: bool,
    recovery: bool,
) -> TestResult<(Heap, Binding<P>)> {
    let [operator, rhs, solution] = crate::solve_testing::shapes(1, 1)?;
    let mut draft = if inverse {
        let output = crate::kernel_testing::shape(
            &operator.declarations,
            operator.codomain(),
            operator.domain(),
        )?;
        OperationDraft::inverse(operator, output, ShapeLimits::default())?
    } else {
        OperationDraft::solve(operator, rhs, solution, ShapeLimits::default())?
    };
    if restored {
        let shape = draft
            .solve
            .as_ref()
            .ok_or("solve")?
            .solution
            .shape()
            .clone();
        let map = shape
            .declarations
            .conversion_map(&Descriptor::ratio(), &Descriptor::ratio())?;
        let conversion = crate::conversion_testing::contract(shape.clone(), shape, map)?;
        draft = draft.restore_solution(conversion)?;
    }
    if recovery {
        draft = draft.with_solve_policy(crate::recovery_testing::policy(
            1.0,
            1,
            nrz_accuracy::Promotion::Never,
            nrz_accuracy::Limits::new(2, 1_000_000),
        )?)?;
    }
    let limits = ExecutionLimits::default();
    let operation = draft.check(limits)?;
    let mut heap = super::recipes::warm::<P>()?;
    let source = heap
        .import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?
        .value();
    let inputs = vec![source; if inverse { 1 } else { 2 }];
    drop(Binding::new(operation.clone(), inputs.clone(), limits).validate(&mut heap)?);
    Ok((heap, Binding::new(operation, inputs, limits)))
}

/// Fixed one-by-one fixture counts include all finite recovery requests, even an unused suffix.
fn expected_bytes<P: Scalar>(inverse: bool, restored: bool, recovery: bool) -> [usize; 2] {
    let [headers, request, input] = crate::solve::scratch::fixture_owner_sizes();
    let attempts = if recovery { 2 } else { 1 };
    let requests = if recovery { 23 } else { 11 } + 2 * attempts * usize::from(restored);
    let metadata = headers + requests * request + if inverse { 1 } else { 2 } * input;
    let width = size_of::<P>();
    let trace = size_of::<crate::ExecutionId>() + 7 * size_of::<usize>();
    let retained = 3 * 64 + trace + metadata;
    // Factor packing drops before the inverse: two 64-byte rectangles and the copied scalar
    // dominate the later inverse-plus-defect vectors. LU keeps its original 256-work allowance.
    let factor = 2 * 64 + width;
    // Each attempt packs two rectangles. Ordinary scalar requests number six, or seven plus
    // nine for recovery; restoration adds two per attempt. Three candidate scalars count once.
    let scalars = if recovery { 16 } else { 6 } + 2 * attempts * usize::from(restored);
    let numerical = factor + 256 * width + 2 * attempts * 64 + (scalars + 3) * width;
    [
        retained,
        retained
            + 3 * size_of::<faer::Mat<P>>()
            + trace
            + numerical
            + 3 * (2 * size_of::<crate::heap::identity::Key>()
                + size_of::<crate::heap::records::Record>())
            + size_of::<crate::publication::Payload<P>>(),
    ]
}

/// Both account ceilings check exact independent neighbours before any reserved numerical owner.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for inverse in [false, true] {
        for restored in [false, true] {
            for recovery in [false, true] {
                let expected = expected_bytes::<P>(inverse, restored, recovery);
                for (staged, needed) in [false, true].into_iter().zip(expected) {
                    for (limit, succeeds) in [(needed - 1, false), (needed, true)] {
                        let (mut heap, binding) = fixture::<P>(inverse, restored, recovery)?;
                        let mut limits = Limits::default();
                        if staged {
                            limits.staged_payload_bytes = u64::try_from(limit)?;
                        } else {
                            limits.output_bytes = u64::try_from(limit)?;
                        }
                        let mut account = ExecutionAccount::new(Budget::new(limits)?)?;
                        let injection = crate::loan::fault::Injection::after(if succeeds {
                            usize::MAX
                        } else {
                            0
                        });
                        match binding.validate_with_account(&mut heap, &mut account) {
                            Ok(bound) => {
                                assert!(succeeds);
                                assert_eq!(
                                    [bound.resource_permit.retained, bound.resource_permit.peak],
                                    expected
                                );
                                let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                                let outcome = bound.publish(candidate)?;
                                assert_eq!(
                                    heap.value(outcome.output()[0].value())?
                                        .region()
                                        .get(0, 0)?,
                                    P::ONE
                                );
                            }
                            Err(error) => {
                                assert!(!succeeds, "{error:?}");
                                assert!(
                                    matches!(error,
                                        OperationError::Heap(HeapError::ResourceBytes { staged: actual, used, limit: ceiling })
                                        if actual == staged && used == u64::try_from(needed)? && ceiling == u64::try_from(limit)?
                                    ),
                                    "{error:?}; {:?}, inverse {inverse}, restored {restored}, recovery {recovery}, expected {expected:?}",
                                    P::PRECISION
                                );
                                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                            }
                        }
                        drop(injection);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Plain, inverse, restored and finite-recovery solves retain unchanged Work and complete byte admission.
/// ´test:unit:loan-solve-physical-byte-neighbours´
#[test]
fn loan_solve_physical_byte_neighbours() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

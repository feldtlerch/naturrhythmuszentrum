// SPDX-License-Identifier: AGPL-3.0-only
//! Affine admission counts newly cached coefficients and borrows previously retained formats.

use super::TestResult;
use crate::{
    Binding, CheckedOperation, ExecutionAccount, ExecutionLimits, Extent, Heap, HeapError,
    OperationDraft, OperationError,
};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_scalar::{Precision, Scalar};

/// Two equal rational maps share one local lowering, even through distinct cold contract caches.
fn operation() -> TestResult<CheckedOperation> {
    let declarations = crate::conversion_testing::owner()?;
    let shape = crate::conversion_testing::shape(&declarations, &Descriptor::ratio(), 1)?;
    let map = declarations.conversion_map(&Descriptor::ratio(), &Descriptor::ratio())?;
    let first = crate::conversion_testing::contract(shape.clone(), shape.clone(), map.clone())?;
    let second = crate::conversion_testing::contract(shape.clone(), shape, map)?;
    Ok(OperationDraft::convert(vec![first, second]).check(ExecutionLimits::default())?)
}

/// Drop the numerical candidate, preserving only the definition's cached coefficient owner.
fn prepare<P: Scalar + 'static>(
    operation: &CheckedOperation,
    evaluate: bool,
) -> TestResult<(Heap, Binding<P>)> {
    let mut heap = super::recipes::warm::<P>()?;
    let source = heap
        .import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?
        .value();
    let bound = Binding::new(
        operation.clone(),
        vec![source; 2],
        ExecutionLimits::default(),
    )
    .validate(&mut heap)?;
    if evaluate {
        drop(bound.evaluate(bound.normalized_inputs()?)?);
    }
    drop(bound);
    Ok((
        heap,
        Binding::new(
            operation.clone(),
            vec![source; 2],
            ExecutionLimits::default(),
        ),
    ))
}

/// State zero is cold, one borrows this format, and two retains only the other format.
fn fixture<P: Scalar + 'static>(state: usize) -> TestResult<(Heap, Binding<P>)> {
    let operation = operation()?;
    if state == 2 {
        match P::PRECISION {
            Precision::F32 => {
                drop(prepare::<f64>(&operation, true)?);
            }
            Precision::F64 => {
                drop(prepare::<f32>(&operation, true)?);
            }
        }
    }
    prepare(&operation, state == 1)
}

/// A coefficient has four scalars and a u64; the shared map adds its pointer, revision and padding.
fn expected_bytes<P: Scalar>(state: usize) -> [usize; 2] {
    let word = size_of::<usize>();
    let coefficient = 4 * size_of::<P>() + size_of::<u64>();
    let shared = (word + 2 * coefficient + size_of::<u32>()).div_ceil(word) * word + 2 * word;
    let cache_entry = 2 * word;
    let cold = if state == 1 { 0 } else { shared + cache_entry };
    let replacement = if state == 2 { 2 * cache_entry } else { 0 };
    let trace = size_of::<crate::ExecutionId>() + 7 * word;
    let retained = 6 * 64 + trace + cold;
    let staged = retained
        + 6 * size_of::<faer::Mat<P>>()
        + 6 * (2 * size_of::<crate::heap::identity::Key>()
            + size_of::<crate::heap::records::Record>())
        + 2 * size_of::<crate::publication::Payload<P>>()
        + trace
        + (6 + 8) * size_of::<P>()
        + 2 * word
        + replacement;
    [retained, staged]
}

/// Retained and staged neighbours distinguish cold ownership, warm borrowing and cache replacement.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for state in 0..3 {
        let expected = expected_bytes::<P>(state);
        for (staged, needed) in [false, true].into_iter().zip(expected) {
            for (limit, succeeds) in [(needed - 1, false), (needed, true)] {
                let (mut heap, binding) = fixture::<P>(state)?;
                let mut limits = Limits::default();
                if staged {
                    limits.staged_payload_bytes = u64::try_from(limit)?;
                } else {
                    limits.output_bytes = u64::try_from(limit)?;
                }
                let mut account = ExecutionAccount::new(Budget::new(limits)?)?;
                let injection =
                    crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
                match binding.validate_with_account(&mut heap, &mut account) {
                    Ok(bound) => {
                        assert!(succeeds);
                        assert_eq!(
                            [bound.resource_permit.retained, bound.resource_permit.peak],
                            expected
                        );
                        // One actual six-record backing; per-bundle preparation appends in place.
                        assert_eq!(crate::loan::fault::Injection::record_backings(), [1, 6]);
                        let assembly =
                            6 * 64 + 64 + 6 * 56 + 12 * 16 + 6 * 24 + 6 * size_of::<P>() + 6 * 248;
                        assert!(assembly <= expected[1]);
                        let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                        let outcome = bound.publish(candidate)?;
                        assert_eq!(outcome.output().len(), 2);
                    }
                    Err(error) => {
                        assert!(!succeeds, "{error:?}");
                        assert!(
                            matches!(error,
                                OperationError::Heap(HeapError::ResourceBytes { staged: actual, used, limit: ceiling })
                                if actual == staged && used == u64::try_from(needed)? && ceiling == u64::try_from(limit)?
                            ),
                            "{error:?}; {:?}, state {state}, expected {expected:?}",
                            P::PRECISION
                        );
                        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                        assert_eq!(crate::loan::fault::Injection::record_backings(), [0; 2]);
                    }
                }
                drop(injection);
            }
        }
    }
    Ok(())
}

/// Shared rational lowerings charge one new coefficient owner; warm owners charge no new payload.
/// ´test:unit:loan-affine-cache-physical-byte-neighbours´
#[test]
fn loan_affine_cache_physical_byte_neighbours() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

/// A single represented copy reserves one `Record` backing while replacing an expired `Trace`.
fn copy_boundaries<P: Scalar + 'static>() -> TestResult {
    for succeeds in [false, true] {
        let mut heap = super::recipes::warm::<P>()?;
        let source = heap
            .import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?
            .value();
        let operation = crate::binding_testing::operation(1, 1)?;
        drop(
            Binding::new(operation.clone(), vec![source], ExecutionLimits::default())
                .validate(&mut heap)?,
        );
        // The assembly phase has one slab, new Trace, Mat, two keys, zero header/scalar and Record.
        let assembly = 64 + 64 + 56 + 2 * 16 + 24 + size_of::<P>() + 248;
        // Admission also retains the disjoint expired-Trace prefix and two-scalar scratch allowance.
        let staged =
            assembly + 64 + 2 * size_of::<P>() + size_of::<crate::publication::Payload<P>>() - 24;
        let limit = staged - usize::from(!succeeds);
        let mut account = ExecutionAccount::new(Budget::new(Limits {
            staged_payload_bytes: u64::try_from(limit)?,
            ..Limits::default()
        })?)?;
        let injection = crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
        let result = Binding::new(operation, vec![source], ExecutionLimits::default())
            .validate_with_account(&mut heap, &mut account);
        match result {
            Ok(bound) => {
                assert!(succeeds);
                assert_eq!(crate::loan::fault::Injection::record_backings(), [1, 1]);
                let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                assert_eq!(candidate.outputs.capacity(), 1);
                let outcome = bound.publish(candidate)?;
                assert_eq!(
                    heap.value(outcome.output()[0].value())?
                        .region()
                        .get(0, 0)?,
                    P::ONE
                );
            }
            Err(error) => {
                assert!(!succeeds);
                assert!(
                    matches!(error,
                    OperationError::Heap(HeapError::ResourceBytes { staged: true, used, limit: ceiling })
                    if used == u64::try_from(staged)? && ceiling == u64::try_from(limit)?),
                    "{error:?}; staged {staged}"
                );
                assert_eq!(crate::loan::fault::Injection::record_backings(), [0; 2]);
            }
        }
        drop(injection);
    }
    Ok(())
}

/// Byte neighbours exercise the actual single-backing assembly and its complete publication.
/// ´test:unit:loan-copy-record-backing-byte-neighbours´
#[test]
fn loan_copy_record_backing_byte_neighbours() -> TestResult {
    // Extent has rows, columns and cached entries; the target adds two words. Three
    // Buffer owners use 56 bytes each, and the optional contribution array uses 72.
    assert_eq!(
        size_of::<crate::publication::Payload<f32>>(),
        3 * 8 + 2 * 8 + 3 * 56 + 72
    );
    assert_eq!(size_of::<crate::publication::Payload<f64>>(), 280);
    copy_boundaries::<f32>()?;
    copy_boundaries::<f64>()
}

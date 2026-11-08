// SPDX-License-Identifier: AGPL-3.0-only
//! Independent physical boundaries for lowered fixed maps and decoded reciprocal schedules.

use super::TestResult;
use crate::{
    Binding, ExecutionAccount, ExecutionLimits, Extent, FixedCoefficient, Heap, HeapError,
    HeapLimits, LinearMap, OperationDraft, OperationError, ShapeLimits,
};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_scalar::{LiteralLimits, Scalar};

/// Resident sources determine occupations, while unrelated payloads do not enter per-call bytes.
pub(super) fn warm<P: Scalar + 'static>() -> TestResult<Heap> {
    let mut heap = Heap::new(HeapLimits::default())?;
    let mut handles = Vec::new();
    for _ in 0..32 {
        handles.push(
            heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?
                .value(),
        );
    }
    for handle in handles {
        heap.retire(handle)?;
    }
    Ok(heap)
}

/// The fixed operator's coefficient count is independent of the input and output vector counts.
fn linear<P: Scalar + 'static>(rows: usize) -> TestResult<(Heap, Binding<P>)> {
    let mut heap = warm::<P>()?;
    let declarations = crate::kernel_testing::owner()?;
    let source = crate::kernel_testing::axis(32)?;
    let result = crate::kernel_testing::axis(rows)?;
    let column = crate::kernel_testing::axis(1)?;
    let operator = crate::kernel_testing::shape(&declarations, &source, &result)?;
    let input = crate::kernel_testing::shape(&declarations, &column, &source)?;
    let map = LinearMap::new(
        operator,
        input,
        vec![FixedCoefficient::represented(Descriptor::ratio(), 1.0)?; rows * 32],
        Vec::new(),
        LiteralLimits::default(),
        ShapeLimits::default(),
    )?;
    let limits = ExecutionLimits::default();
    let operation = OperationDraft::linear_map(map).check(limits)?;
    let value = heap
        .import_represented(Extent::new(32, 1)?, &Descriptor::ratio(), &[P::ONE; 32])?
        .value();
    drop(Binding::new(operation.clone(), vec![value], limits).validate(&mut heap)?);
    Ok((heap, Binding::new(operation, vec![value], limits)))
}

/// Exact LC preparation is complete before the smaller execution account is constructed.
fn reciprocal<P: Scalar + 'static>() -> TestResult<(Heap, Binding<P>)> {
    let (prepared, recipe) = crate::reciprocal_testing::proved()?;
    let limits = ExecutionLimits::default();
    let operation = crate::reciprocal_testing::draft(&prepared, &recipe)?.check(limits)?;
    let mut heap = warm::<P>()?;
    let mut inputs = Vec::new();
    let denominators = [
        P::from_u8(40),
        P::from_u8(100) * P::from_u8(100) * P::from_u8(10),
    ];
    for (entry, denominator) in recipe.frame().entries().iter().zip(denominators) {
        inputs.push(
            heap.import_represented(
                Extent::new(1, 1)?,
                entry.descriptor(),
                &[P::ONE / denominator],
            )?
            .value(),
        );
    }
    drop(Binding::new(operation.clone(), inputs.clone(), limits).validate(&mut heap)?);
    Ok((heap, Binding::new(operation, inputs, limits)))
}

/// A warmed three-channel result owns three rounded slabs and one new Trace header.
fn bytes<P: Scalar>(entries: usize, numerical_peak: usize) -> [usize; 2] {
    let slabs = 3 * (entries * size_of::<P>()).div_ceil(64) * 64;
    let trace = size_of::<crate::ExecutionId>() + 7 * size_of::<usize>();
    [
        slabs + trace,
        slabs
            + 3 * size_of::<faer::Mat<P>>()
            + 2 * trace
            + numerical_peak
            + 3 * (2 * size_of::<crate::heap::identity::Key>()
                + size_of::<crate::heap::records::Record>())
            + size_of::<crate::publication::Payload<P>>(),
    ]
}

/// An allocation-site counter proves refusal precedes coefficient allocation, then observes success.
fn boundary<P: Scalar + 'static>(
    fixture: impl Fn() -> TestResult<(Heap, Binding<P>)>,
    expected: [usize; 2],
    ceilings: &[(usize, bool)],
    allocations: [usize; 2],
) -> TestResult {
    for &(limit, succeeds) in ceilings {
        let (mut heap, binding) = fixture()?;
        let mut account = ExecutionAccount::new(Budget::new(Limits {
            output_bytes: u64::try_from(expected[0])?,
            staged_payload_bytes: u64::try_from(limit)?,
            ..Limits::default()
        })?)?;
        let injection = crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
        match binding.validate_with_account(&mut heap, &mut account) {
            Ok(bound) => {
                assert!(succeeds);
                assert_eq!(
                    [bound.resource_permit.retained, bound.resource_permit.peak],
                    expected
                );
                assert_eq!(crate::loan::fault::Injection::kernel_allocations(), [0; 2]);
                let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                assert_eq!(
                    crate::loan::fault::Injection::kernel_allocations(),
                    allocations
                );
                let outcome = bound.publish(candidate)?;
                assert_eq!(outcome.output().len(), 1);
            }
            Err(error) => {
                assert!(!succeeds, "{error:?}");
                assert!(matches!(error,
                    OperationError::Heap(HeapError::ResourceBytes { staged: true, used, limit: ceiling })
                    if used == u64::try_from(expected[1])? && ceiling == u64::try_from(limit)?
                ));
                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                assert_eq!(crate::loan::fault::Injection::kernel_allocations(), [0; 2]);
            }
        }
        drop(injection);
    }
    Ok(())
}

/// Each coefficient and result entry has one center and two independent scalar fields.
fn linear_boundaries<P: Scalar + 'static>(rows: usize) -> TestResult {
    let width = size_of::<P>();
    let lowered = rows * 32 * 3 * width;
    let entries = rows * 3 * width;
    let expected = bytes::<P>(rows, (lowered + entries).max(entries + 3 * rows * width));
    boundary(
        || linear::<P>(rows),
        expected,
        &[
            (16_384, expected[1] <= 16_384),
            (expected[1] - 1, false),
            (expected[1], true),
        ],
        [1, 0],
    )
}

/// The binary32 32-by-32 case fits 16,384 bytes; its 64-by-32 neighbour and binary64 do not.
/// ´test:unit:loan-linear-coefficient-byte-neighbours´
#[test]
fn loan_linear_coefficient_byte_neighbours() -> TestResult {
    linear_boundaries::<f32>(32)?;
    linear_boundaries::<f32>(64)?;
    linear_boundaries::<f64>(32)
}

/// Fourteen decoded literals (four scalars and a u64), two five-scalar images and four entries.
fn reciprocal_boundaries<P: Scalar + 'static>() -> TestResult {
    let width = size_of::<P>();
    let decoded = 14 * (4 * width + size_of::<u64>());
    let images = 2 * 5 * width;
    let entries = 4 * 3 * width;
    let expected = bytes::<P>(4, (decoded + images + entries).max(entries + 12 * width));
    boundary(
        reciprocal::<P>,
        expected,
        &[(800, false), (expected[1] - 1, false), (expected[1], true)],
        [0, 3],
    )
}

/// Both LC precisions refuse 800 staged bytes before any numerical literal owner is allocated.
/// ´test:unit:loan-reciprocal-decoded-byte-neighbours´
#[test]
fn loan_reciprocal_decoded_byte_neighbours() -> TestResult {
    reciprocal_boundaries::<f32>()?;
    reciprocal_boundaries::<f64>()
}

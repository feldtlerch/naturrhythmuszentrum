// SPDX-License-Identifier: AGPL-3.0-only
//! Ordinary products admit the physical packing peak before entering the allocating backend.

use super::TestResult;
use crate::{
    Binding, ExecutionAccount, ExecutionLimits, Extent, Heap, HeapError, HeapLimits,
    OperationDraft, OperationError, ShapeLimits,
};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_scalar::Scalar;

/// Warm spare tables and an expired trace, then bind two represented ratio rectangles.
fn fixture<P: Scalar + 'static>(dimensions: [usize; 3]) -> TestResult<(Heap, Binding<P>)> {
    let mut heap = Heap::new(HeapLimits::default())?;
    let mut warm = Vec::new();
    for _ in 0..32 {
        warm.push(
            heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?
                .value(),
        );
    }
    for handle in warm {
        heap.retire(handle)?;
    }
    let [rows, contracted, columns] = dimensions;
    let declarations = crate::kernel_testing::owner()?;
    let output = crate::kernel_testing::axis(rows)?;
    let middle = crate::kernel_testing::axis(contracted)?;
    let input = crate::kernel_testing::axis(columns)?;
    let first = crate::kernel_testing::shape(&declarations, &middle, &output)?;
    let second = crate::kernel_testing::shape(&declarations, &input, &middle)?;
    let operation = OperationDraft::matmul(first, second, ShapeLimits::default())?
        .check(ExecutionLimits::default())?;
    let left = heap
        .import_represented(
            Extent::new(rows, contracted)?,
            &Descriptor::ratio(),
            &vec![P::ONE; rows * contracted],
        )?
        .value();
    let right = heap
        .import_represented(
            Extent::new(contracted, columns)?,
            &Descriptor::ratio(),
            &vec![P::ONE; contracted * columns],
        )?
        .value();
    drop(
        Binding::new(
            operation.clone(),
            vec![left, right],
            ExecutionLimits::default(),
        )
        .validate(&mut heap)?,
    );
    Ok((
        heap,
        Binding::new(operation, vec![left, right], ExecutionLimits::default()),
    ))
}

/// Independent owner arithmetic covers packing-dominated and scalar-splitting-dominated products.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for dimensions in [[1, 256, 1], [32, 1, 32]] {
        let [rows, contracted, columns] = dimensions;
        let width = size_of::<P>();
        let entries = rows * columns;
        let rectangle = |rows: usize, columns| (rows * width).div_ceil(64) * 64 * columns;
        // Six final slabs and six temporary Mat headers; tables already have spare capacity.
        let destinations = 6 * (entries * width).div_ceil(64) * 64;
        // Trace has the execution identity, five counters and two reference-count words.
        let trace = size_of::<crate::ExecutionId>() + 7 * size_of::<usize>();
        let retained = destinations + trace;
        // Packed operands and backend result coexist with copied centers. Later phases own
        // 7 scalars (verification), 12 (splitting), or 9 (final channels) per output entry.
        let packed = rectangle(rows, contracted)
            + rectangle(contracted, columns)
            + rectangle(rows, columns)
            + entries * width;
        let transient = packed
            .max(7 * entries * width)
            .max(12 * entries * width)
            .max(9 * entries * width);
        let staged = destinations
            + 6 * size_of::<faer::Mat<P>>()
            + 2 * trace
            + transient
            + 6 * (2 * size_of::<crate::heap::identity::Key>()
                + size_of::<crate::heap::records::Record>())
            + size_of::<crate::publication::Payload<P>>();
        for (limit, succeeds) in [(16_384, false), (staged - 1, false), (staged, true)] {
            let (mut heap, binding) = fixture::<P>(dimensions)?;
            let mut account = ExecutionAccount::new(Budget::new(Limits {
                output_bytes: u64::try_from(retained)?,
                staged_payload_bytes: u64::try_from(limit)?,
                ..Limits::default()
            })?)?;
            let injection =
                crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
            match binding.validate_with_account(&mut heap, &mut account) {
                Ok(bound) => {
                    assert!(succeeds);
                    assert_eq!(bound.resource_permit.retained, retained);
                    assert_eq!(bound.resource_permit.peak, staged);
                    assert_eq!(crate::loan::fault::Injection::product_backends(), 0);
                    let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                    assert_eq!(crate::loan::fault::Injection::product_backends(), 1);
                    let result = bound.publish(candidate)?;
                    let output = result.output().first().ok_or("missing product")?;
                    assert_eq!(
                        heap.value(output.value())?.region().get(0, 0)?.to_f64(),
                        f64::from(u32::try_from(contracted)?)
                    );
                }
                Err(error) => {
                    assert!(!succeeds, "{error:?}");
                    assert!(matches!(error,
                        OperationError::Heap(HeapError::ResourceBytes { staged: true, used, limit: ceiling })
                        if used == u64::try_from(staged)? && ceiling == u64::try_from(limit)?
                    ));
                    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                    assert_eq!(crate::loan::fault::Injection::product_backends(), 0);
                }
            }
            drop(injection);
        }
    }
    Ok(())
}

/// The review's 1-by-256 by 256-by-1 product refuses 16,384 staged bytes before backend entry.
/// ´test:unit:loan-product-rounded-packing-byte-neighbours´
#[test]
fn loan_product_rounded_packing_byte_neighbours() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

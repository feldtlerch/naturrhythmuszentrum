// SPDX-License-Identifier: AGPL-3.0-only
//! Two growing reservations retain final backing and charge only the largest real replacement.

use super::{Case, TestResult};
use crate::{
    ExecutionAccount, ExecutionLimits, HeapError, OperationDraft, OperationError, ShapeLimits,
};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{Enclosure, Scalar};

/// The repeated four-source addition has 188 paired entries and 128 publication-copy scalars.
fn expected_bytes<P: Scalar>() -> [usize; 2] {
    use crate::heap::{descriptors::DescriptorRecord, identity::Key};
    let width = size_of::<P>();
    let metadata = crate::statistics::inventory::addition_fixture_metadata_bytes();
    let paired = 16 + 4 + 16 + 4 + 32 + 16 + 32 + 4 + 16 + 16 + 16 + 16;
    let vectors = paired * (width + size_of::<Enclosure<P>>()) + 2 * (16 + 16 + 32) * width;
    // Sensitivity packs 4-by-8, 8-by-4, and 4-by-4; the next two contractions each pack three 4-by-4s.
    let packed = (8 + 4 + 4 + 2 * 3 * 4) * 64;
    let slabs = 3 * 64 + 4 * (16 * width).div_ceil(64) * 64 + 2 * (32 * width).div_ceil(64) * 64;
    let trace = size_of::<crate::ExecutionId>() + 7 * size_of::<usize>();
    let slots =
        size_of::<(u64, Option<DescriptorRecord>)>() + size_of::<(u64, Option<faer::Mat<P>>)>();
    let retained = slabs + metadata + trace + (16 - 4) * slots;
    // Three resident occupations plus three ordinary outputs grow 4 to 8. The nine-output
    // statistical replacement grows 8 to 16. Peaks are 4+8 and 8+16, never their sum.
    let staged = slabs
        + super::header_bytes::<P>(2)
        + (3 + 9) * size_of::<crate::heap::records::Record>()
        + size_of::<crate::publication::Payload<P>>()
        + metadata
        + 2 * trace
        + (3 + 9) * size_of::<faer::Mat<P>>()
        + (3 + 9) * 2 * size_of::<Key>()
        + 3 * 64
        + 12 * width
        + (8 + 12) * width
        + vectors
        + packed
        + 3008 * width
        + (8 + 16) * slots;
    [retained, staged]
}

/// Account boundaries are derived from fixture counts and layouts, never from a prepared plan.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    let expected = expected_bytes::<P>();
    for (staged, needed) in [false, true].into_iter().zip(expected) {
        let mut cases = vec![(needed - 1, false), (needed, true)];
        if staged {
            cases.push((50_000, expected[1] <= 50_000));
        }
        for (limit, succeeds) in cases {
            let mut case = Case::<P>::new(false)?;
            let shape = case.operation.inputs()[0].shape().clone();
            case.operation = OperationDraft::add(shape.clone(), shape, ShapeLimits::default())?
                .check(ExecutionLimits::default())?;
            let mut limits = Limits::default();
            if staged {
                limits.staged_payload_bytes = u64::try_from(limit)?;
            } else {
                limits.output_bytes = u64::try_from(limit)?;
            }
            let mut account = ExecutionAccount::new(Budget::new(limits)?)?;
            let injection =
                crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
            match case
                .binding()
                .validate_with_account(&mut case.heap, &mut account)
            {
                Ok(bound) => {
                    assert!(succeeds);
                    assert_eq!(bound.statistical_work(), 3008);
                    assert_eq!(
                        [bound.resource_permit.retained, bound.resource_permit.peak],
                        expected
                    );
                    let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                    let outcome = bound.publish(candidate)?;
                    assert_eq!(outcome.output().len(), 1);
                    assert_eq!(
                        crate::loan::fault::Injection::statistical_headers(),
                        [4, super::header_bytes::<P>(2)]
                    );
                }
                Err(error) => {
                    assert!(!succeeds, "{error:?}");
                    assert!(
                        matches!(error,
                            OperationError::Heap(HeapError::ResourceBytes { staged: actual, used, limit: ceiling })
                            if actual == staged && used == u64::try_from(needed)? && ceiling == u64::try_from(limit)?
                        ),
                        "{error:?}; precision {:?}, staged {staged}, limit {limit}, expected {expected:?}",
                        P::PRECISION
                    );
                    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                    assert_eq!(crate::loan::fault::Injection::statistical_headers(), [0; 2]);
                }
            }
            drop(injection);
        }
    }
    Ok(())
}

/// Both precisions check 50,000 bytes and exact neighbours with physical record and zero owners included.
/// ´test:unit:loan-statistical-sequential-growth-byte-neighbours´
#[test]
fn loan_statistical_sequential_growth_byte_neighbours() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

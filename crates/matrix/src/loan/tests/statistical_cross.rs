// SPDX-License-Identifier: AGPL-3.0-only
//! Standalone cross covariance admits its complete schedule before numerical capture.

use super::{Case, TestResult};
use crate::{HeapError, OperationError};
use nrz_bindings::{Budget, Limits};
use nrz_scalar::{Enclosure, Scalar};

/// Twenty-two requests cover seven paired 4-by-4 arrays, two contractions and two copies.
fn expected<P: Scalar>() -> [usize; 2] {
    let width = size_of::<P>();
    let metadata = crate::statistics::inventory::cross_fixture_metadata_bytes();
    let retained = metadata + 2 * (16 * width).div_ceil(64) * 64;
    let paired = 7 * 16 * (width + size_of::<Enclosure<P>>());
    let copies = 2 * 16 * width;
    let rectangles = 2 * 3 * 4 * 64;
    let staged = retained
        + paired
        + copies
        + rectangles
        + 2304 * width
        + 2 * size_of::<faer::Mat<P>>()
        + 4 * size_of::<crate::heap::identity::Key>()
        + 2 * size_of::<crate::heap::records::Record>();
    [retained, staged]
}

/// Exact ceilings are independent of the metadata-only compatibility policy supplied by callers.
fn boundaries<P: Scalar + 'static>() -> TestResult {
    for (staged, needed) in [false, true].into_iter().zip(expected::<P>()) {
        for (limit, succeeds) in [(needed - 1, false), (needed, true)] {
            let mut case = Case::<P>::new(true)?;
            let mut policy = crate::heap::resources::ResourcePolicy::compatibility();
            if staged {
                policy.staged = u64::try_from(limit)?;
            } else {
                policy.output = u64::try_from(limit)?;
            }
            let injection =
                crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
            let result = case.heap.cross_with_policy(
                case.input,
                case.input,
                crate::JointLimits::default(),
                &Budget::new(Limits::default())?,
                policy,
            );
            match result {
                Ok(_) => {
                    assert!(succeeds);
                    assert_eq!(case.heap.descriptors.iter().count(), 5);
                }
                Err(error) => {
                    assert!(!succeeds, "{error:?}");
                    assert!(
                        matches!(error,
                            OperationError::Heap(HeapError::ResourceBytes { staged: actual, used, limit: ceiling })
                            if actual == staged && used == u64::try_from(needed)? && ceiling == u64::try_from(limit)?
                        ),
                        "{error:?}; expected {:?}",
                        expected::<P>()
                    );
                    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                    assert_eq!(case.heap.descriptors.iter().count(), 3);
                }
            }
            drop(injection);
        }
    }
    Ok(())
}

/// Each retained and staged N-1 refusal precedes capture; each N result reaches publication.
/// ´test:unit:loan-standalone-cross-physical-byte-neighbours´
#[test]
fn loan_standalone_cross_physical_byte_neighbours() -> TestResult {
    boundaries::<f32>()?;
    boundaries::<f64>()
}

/// Full descriptor and arena tables cannot receive two separate output ceilings in a cross call.
/// ´test:unit:loan-standalone-cross-combined-ceiling´
#[test]
fn loan_standalone_cross_combined_ceiling() -> TestResult {
    let mut case = Case::<f64>::new(true)?;
    case.heap.limits = crate::HeapLimits::new(8192, 1_048_576, 4096);
    for _ in 3..4096 {
        case.heap.import_represented(
            crate::Extent::new(1, 1)?,
            &nrz_bindings::Descriptor::ratio(),
            &[1.0_f64],
        )?;
    }
    let rows = size_of::<(u64, Option<crate::heap::descriptors::DescriptorRecord>)>()
        + size_of::<(u64, Option<faer::Mat<f64>>)>();
    let needed = expected::<f64>()[0] + 4096 * rows;
    let before = [
        case.heap.descriptors.slots.owned_bytes()?,
        case.heap.storage.owned_bytes()?,
    ];
    let injection = crate::loan::fault::Injection::after(0);
    let result = case.heap.materialize_cross_covariance(
        case.input,
        case.input,
        crate::JointLimits::default(),
    );
    assert!(
        matches!(result,
            Err(OperationError::Heap(HeapError::ResourceBytes { staged: false, used, limit: 1_048_576 }))
            if used == u64::try_from(needed)?
        ),
        "{result:?}"
    );
    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
    drop(injection);
    assert_eq!(
        [
            case.heap.descriptors.slots.owned_bytes()?,
            case.heap.storage.owned_bytes()?
        ],
        before
    );
    Ok(())
}

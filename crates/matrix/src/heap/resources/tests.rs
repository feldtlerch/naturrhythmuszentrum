// SPDX-License-Identifier: AGPL-3.0-only
//! Independent legacy table and slab boundaries exercise complete per-call admission.

use crate::{Extent, Handle, Heap, HeapError, HeapLimits, Value};
use nrz_bindings::{Descriptor, Limits};

/// Fill both geometric slot tables while reusing one scalar schema.
fn full() -> Result<(Heap, Handle<Value, f64>), HeapError> {
    let mut heap = Heap::new(HeapLimits::new(8192, 1_048_576, 4096))?;
    let extent = Extent::new(1, 1)?;
    let quantity = Descriptor::ratio();
    let first = heap
        .import_represented(extent, &quantity, &[1.0_f64])?
        .value();
    for _ in 1..4096 {
        heap.import_represented(extent, &quantity, &[1.0_f64])?;
    }
    Ok((heap, first))
}

/// Both isolated table subtotals fit; their sum and the new slabs cross the one output ceiling.
/// ´test:unit:loan-legacy-import-replacement-combined-ceiling´
#[test]
fn loan_legacy_import_replacement_combined_ceiling() -> Result<(), HeapError> {
    let (mut heap, first) = full()?;
    let descriptor = size_of::<(u64, Option<crate::heap::descriptors::DescriptorRecord>)>();
    let arena = size_of::<(u64, Option<faer::Mat<f64>>)>();
    let ceiling = Limits::default().output_bytes;
    let growth = 4096 * (descriptor + arena);
    assert_eq!(growth + 64, 1_310_784);
    assert!(u64::try_from(4096 * descriptor).map_err(|_| HeapError::Capacity)? <= ceiling);
    assert!(u64::try_from(4096 * arena + 128).map_err(|_| HeapError::Capacity)? < ceiling);
    let before = [
        heap.descriptors.slots.owned_bytes()?,
        heap.storage.owned_bytes()?,
    ];
    let stamp = heap.value(first)?.stamp();
    for replace in [false, true] {
        let injection = crate::loan::fault::Injection::after(0);
        let result = if replace {
            heap.replace_declared_bounds(first, stamp.version(), &[2.0], &[0.5])
        } else {
            heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[2.0_f64])
        };
        let needed = growth + if replace { 128 } else { 64 };
        assert!(
            matches!(result,
                Err(HeapError::ResourceBytes { staged: false, used, limit })
                if used == u64::try_from(needed).map_err(|_| HeapError::Capacity)? && limit == ceiling
            ),
            "{result:?}"
        );
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
        assert_eq!(
            [
                heap.descriptors.slots.owned_bytes()?,
                heap.storage.owned_bytes()?
            ],
            before
        );
        assert_eq!(heap.value(first)?.stamp(), stamp);
        assert_eq!(
            heap.value(first)?.region().get(0, 0)?.to_bits(),
            1.0_f64.to_bits()
        );
    }
    // A vacancy removes both table replacements; the unrelated 4,095 resident owners stay excluded.
    heap.retire(first)?;
    heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[3.0_f64])?;
    assert_eq!(
        [
            heap.descriptors.slots.owned_bytes()?,
            heap.storage.owned_bytes()?
        ],
        before
    );
    Ok(())
}

/// Joint declarations include their parameter descriptors in the same call as their slabs.
/// ´test:unit:loan-joint-publication-combined-ceiling´
#[test]
fn loan_joint_publication_combined_ceiling() -> Result<(), Box<dyn std::error::Error>> {
    let (mut heap, first) = full()?;
    let limits = crate::JointLimits::default();
    let covariance = crate::DenseCovariance::new(
        1,
        &[1.0_f64],
        &nrz_scalar::ArithmeticContext::new()?,
        limits,
    )?;
    let model = crate::JointModel::from_dense(
        &[crate::Source::new(&Descriptor::ratio(), 1)?],
        covariance,
        &[crate::CovarianceBlock::Known],
        limits,
    )?;
    let target = crate::TargetMap::capture(&heap, first)?;
    let before = [
        heap.descriptors.slots.owned_bytes()?,
        heap.storage.owned_bytes()?,
    ];
    let injection = crate::loan::fault::Injection::after(0);
    let result = heap.publish_joint(
        crate::CompanionSetDraft::new(model, [target.clone(), target], [vec![0], vec![0]]),
        limits,
    );
    assert!(
        matches!(
            result,
            Err(crate::JointError::Heap(HeapError::ResourceBytes {
                staged: false,
                used: 1_310_848,
                limit: 1_048_576
            }))
        ),
        "{result:?}"
    );
    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
    drop(injection);
    assert_eq!(
        [
            heap.descriptors.slots.owned_bytes()?,
            heap.storage.owned_bytes()?
        ],
        before
    );
    assert_eq!(
        heap.value(first)?.region().get(0, 0)?.to_bits(),
        1.0_f64.to_bits()
    );
    Ok(())
}

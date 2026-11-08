// SPDX-License-Identifier: AGPL-3.0-only
//! Exact cumulative Work and byte boundaries preserve the original caller policy.

use super::{TestResult, assert_mask, fixture};
use crate::MaskError;
use nrz_bindings::{
    Budget, Limits,
    readings::algebra::{FaultKind, limits::BudgetKind},
};

/// Allocate against one changed byte ceiling after preparing the same ordinary input layout.
fn allocation(limit: u64, staged: bool) -> TestResult<bool> {
    let (mut heap, layout, _, _) = fixture(65)?;
    let mut limits = Limits::default();
    if staged {
        limits.staged_payload_bytes = limit;
    } else {
        limits.output_bytes = limit;
    }
    let mut budget = Budget::new(limits.clone())?;
    let result = heap.create_mask(layout, &[1, 64], &mut budget);
    assert_eq!(budget.limits(), &limits);
    match result {
        Ok(version) => {
            assert_mask(&heap, version, &[1, 64].into_iter().collect())?;
            Ok(true)
        }
        Err(MaskError::Budget(fault)) => {
            let expected = if staged {
                BudgetKind::StagedPayloadBytes
            } else {
                BudgetKind::OutputBytes
            };
            assert!(
                matches!(fault.kind, FaultKind::Exhausted { budget, .. } if budget == expected)
            );
            assert_eq!(heap.masks.masks.iter().count(), 0);
            Ok(false)
        }
        Err(error) => Err(error.into()),
    }
}

/// Search the byte boundary without changing defaults, then test N and N-1 explicitly.
/// ´test:unit:mask-reservation-exact-byte-boundaries´
#[test]
fn mask_reservation_exact_byte_boundaries() -> TestResult {
    for staged in [true, false] {
        let mut low = 0;
        let mut high = if staged {
            Limits::default().staged_payload_bytes
        } else {
            Limits::default().output_bytes
        };
        assert!(allocation(high, staged)?);
        while low + 1 < high {
            let middle = low + (high - low) / 2;
            if allocation(middle, staged)? {
                high = middle;
            } else {
                low = middle;
            }
        }
        assert!(allocation(high, staged)?);
        assert!(!allocation(high - 1, staged)?);
    }
    Ok(())
}

/// A failed late Work charge publishes no prefix and restores nested byte reservations.
/// ´test:unit:mask-cumulative-work-and-reservation-restoration´
#[test]
fn mask_cumulative_work_and_reservation_restoration() -> TestResult {
    let (mut heap, layout, correspondence, mut preparation) = fixture(65)?;
    let branch = heap.create_mask(layout, &[1, 7, 63, 64], &mut preparation)?;
    let mut measured = Budget::new(Limits::default())?;
    heap.compact_mask(correspondence, branch, 8, 4, &mut measured)?;
    let needed = measured.usage().work;
    for (limit, success) in [(needed, true), (needed - 1, false)] {
        let (mut heap, layout, correspondence, mut preparation) = fixture(65)?;
        let branch = heap.create_mask(layout, &[1, 7, 63, 64], &mut preparation)?;
        let limits = Limits {
            work: limit + 3,
            ..Limits::default()
        };
        let mut budget = Budget::new(limits.clone())?;
        budget.work(3, super::super::AT)?;
        let before = (
            heap.masks.layouts.iter().count(),
            heap.masks.masks.iter().count(),
        );
        let result = heap.compact_mask(correspondence, branch, 8, 4, &mut budget);
        assert_eq!(result.is_ok(), success);
        assert_eq!(budget.limits(), &limits);
        assert!(budget.usage().work <= limit + 3);
        if !success {
            assert!(matches!(result, Err(MaskError::Budget(_))));
            assert_eq!(
                (
                    heap.masks.layouts.iter().count(),
                    heap.masks.masks.iter().count()
                ),
                before
            );
        }
        assert_mask(&heap, branch, &[1, 7, 63, 64].into_iter().collect())?;
    }
    Ok(())
}

/// Retirement retains no new owner even when existing native backing exceeds the caller ceiling.
/// ´test:unit:mask-retirement-allows-zero-output-bytes´
#[test]
fn mask_retirement_allows_zero_output_bytes() -> TestResult {
    let (mut heap, layout, _, mut preparation) = fixture(65)?;
    let mask = heap.create_mask(layout, &[1, 64], &mut preparation)?;
    let mut budget = Budget::new(Limits {
        output_bytes: 0,
        staged_payload_bytes: u64::try_from(size_of::<crate::heap::identity::Key>())?,
        ..Limits::default()
    })?;
    heap.retire_mask(mask, &mut budget)?;
    assert!(matches!(
        heap.mask(mask, layout),
        Err(MaskError::Heap(crate::HeapError::StaleHandle))
    ));
    Ok(())
}

/// Unrelated resident words do not consume the new mask's retained or staged ceiling.
/// ´test:unit:mask-small-allocation-beside-large-unrelated-mask´
#[test]
fn mask_small_allocation_beside_large_unrelated_mask() -> TestResult {
    let (mut heap, large_layout, _, mut preparation) = fixture(65_536)?;
    let large = heap.create_mask(large_layout, &[65_535], &mut preparation)?;
    let axis = heap.create_cell_axis(65, &mut preparation)?;
    let small_layout = heap.create_cell_layout(
        axis,
        crate::Layout::contiguous(crate::Extent::new(65, 3)?)?,
        &mut preparation,
    )?;
    let mut budget = Budget::new(Limits {
        output_bytes: 1024,
        staged_payload_bytes: 1024,
        ..Limits::default()
    })?;
    let small = heap.create_mask(small_layout, &[1, 64], &mut budget)?;
    assert_mask(&heap, small, &[1, 64].into_iter().collect())?;
    assert_mask(&heap, large, &std::iter::once(65_535).collect())?;
    Ok(())
}

/// The old and copied 4,096-entry gathers coexist until the replacement is published.
/// ´test:unit:mask-layout-replacement-counts-old-gather´
#[test]
fn mask_layout_replacement_counts_old_gather() -> TestResult {
    use crate::heap::identity::Key;
    use crate::mask::storage::{LayoutRecord, Native};
    let header = 2 * size_of::<usize>() + size_of::<LayoutRecord>();
    let gather = 4096 * size_of::<usize>();
    let retained = header + gather;
    let staged = 2 * (header + gather) + 2 * size_of::<Key>() + size_of::<Native<LayoutRecord>>();
    for (output, peak, succeeds) in [
        (49_152, 49_152, false),
        (retained - 1, staged, false),
        (retained, staged - 1, false),
        (retained, staged, true),
    ] {
        let (mut heap, layout, correspondence, mut preparation) = fixture(4096)?;
        let selected: Vec<_> = (0..4096).collect();
        let branch = heap.create_mask(layout, &selected, &mut preparation)?;
        let compacted = heap.compact_mask(correspondence, branch, 4096, 4096, &mut preparation)?;
        let old = compacted.layout();
        let tables = heap.masks.layouts.owned_bytes()?;
        let limits = Limits {
            output_bytes: u64::try_from(output)?,
            staged_payload_bytes: u64::try_from(peak)?,
            ..Limits::default()
        };
        let mut budget = Budget::new(limits.clone())?;
        let replacement = heap.replace_cell_layout(
            old,
            crate::Layout::contiguous(crate::Extent::new(4096, 1)?)?,
            &mut budget,
        );
        match replacement {
            Ok(new) => {
                assert!(succeeds);
                let record = heap.masks.layout(heap.scope, new)?;
                assert_eq!(
                    record.gather.as_ref().ok_or("missing gather")?.indices,
                    selected
                );
                assert!(matches!(
                    heap.mask(compacted, old),
                    Err(MaskError::Heap(crate::HeapError::StaleHandle))
                ));
            }
            Err(MaskError::Budget(fault)) => {
                assert!(!succeeds);
                let (expected_kind, expected_used, expected_limit) = if output < retained {
                    (BudgetKind::OutputBytes, retained, output)
                } else {
                    (BudgetKind::StagedPayloadBytes, staged, peak)
                };
                assert!(matches!(fault.kind,
                    FaultKind::Exhausted { budget, needed: used, limit, .. }
                    if budget == expected_kind && used == u64::try_from(expected_used)?
                        && limit == u64::try_from(expected_limit)?
                ));
                assert_mask(&heap, compacted, &selected.into_iter().collect())?;
            }
            Err(error) => return Err(error.into()),
        }
        assert_eq!(heap.masks.layouts.owned_bytes()?, tables);
        assert_eq!(budget.limits(), &limits);
    }
    Ok(())
}

/// A full 1,024-slot table reuses the replaced occupation without staging unrelated backing.
/// ´test:unit:mask-layout-replacement-reuses-full-table´
#[test]
fn mask_layout_replacement_reuses_full_table() -> TestResult {
    use crate::heap::identity::Key;
    use crate::mask::storage::{LayoutRecord, Native};
    let header = 2 * size_of::<usize>() + size_of::<LayoutRecord>();
    let staged = 2 * header + 2 * size_of::<Key>() + size_of::<Native<LayoutRecord>>();
    assert!(staged < 1024);
    let (mut heap, old, _, mut preparation) = fixture(1)?;
    for _ in 1..1024 {
        let axis = heap.create_cell_axis(1, &mut preparation)?;
        heap.create_cell_layout(
            axis,
            crate::Layout::contiguous(crate::Extent::new(1, 1)?)?,
            &mut preparation,
        )?;
    }
    let tables = heap.masks.layouts.owned_bytes()?;
    assert_eq!(heap.masks.layouts.scan_work(), 1024);
    assert!(tables > 1024);
    let mut current = old;
    for (output, peak, succeeds) in [
        (header - 1, 1024, false),
        (header, staged - 1, false),
        (header, staged, true),
        (1024, 1024, true),
    ] {
        let mut budget = Budget::new(Limits {
            output_bytes: u64::try_from(output)?,
            staged_payload_bytes: u64::try_from(peak)?,
            ..Limits::default()
        })?;
        let result = heap.replace_cell_layout(
            current,
            crate::Layout::contiguous(crate::Extent::new(1, 1)?)?,
            &mut budget,
        );
        match result {
            Ok(new) => {
                assert!(succeeds);
                assert!(matches!(
                    heap.masks.layout(heap.scope, current),
                    Err(MaskError::Heap(crate::HeapError::StaleHandle))
                ));
                current = new;
            }
            Err(MaskError::Budget(fault)) => {
                assert!(!succeeds);
                let (kind, expected, ceiling) = if output < header {
                    (BudgetKind::OutputBytes, header, output)
                } else {
                    (BudgetKind::StagedPayloadBytes, staged, peak)
                };
                assert!(matches!(fault.kind,
                    FaultKind::Exhausted { budget, needed: used, limit, .. }
                    if budget == kind && used == u64::try_from(expected)?
                        && limit == u64::try_from(ceiling)?
                ));
                heap.masks.layout(heap.scope, current)?;
            }
            Err(error) => return Err(error.into()),
        }
        assert_eq!(heap.masks.layouts.owned_bytes()?, tables);
    }
    Ok(())
}

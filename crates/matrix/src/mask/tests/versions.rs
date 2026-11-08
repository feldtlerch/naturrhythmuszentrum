// SPDX-License-Identifier: AGPL-3.0-only
//! Equal shapes and bytes do not authorize a foreign or retired occupation.

use super::{TestResult, assert_mask, fixture};
use crate::{Extent, Heap, HeapError, HeapLimits, Layout, MaskError};

/// Independently published layouts refuse every algebra route despite identical dimensions.
/// ´test:unit:mask-unrelated-layout-refusal´
#[test]
fn mask_unrelated_layout_refusal() -> TestResult {
    let (mut heap, layout, correspondence, mut budget) = fixture(65)?;
    let left = heap.create_mask(layout, &[0, 64], &mut budget)?;
    let axis = heap.mask(left, layout)?.axis();
    let other_layout =
        heap.create_cell_layout(axis, Layout::contiguous(Extent::new(65, 3)?)?, &mut budget)?;
    let right = heap.create_mask(other_layout, &[0, 64], &mut budget)?;
    assert!(matches!(
        heap.mask(left, other_layout),
        Err(MaskError::LayoutMismatch)
    ));
    assert!(matches!(
        heap.mask_correspondence(layout, other_layout, &mut budget),
        Err(MaskError::NonIdentityCorrespondence)
    ));
    assert!(matches!(
        heap.intersect_masks(correspondence, left, right, &mut budget),
        Err(MaskError::LayoutMismatch)
    ));
    assert!(matches!(
        heap.union_masks(correspondence, left, right, &mut budget),
        Err(MaskError::LayoutMismatch)
    ));
    assert!(matches!(
        heap.complement_mask(correspondence, left, right, &mut budget),
        Err(MaskError::LayoutMismatch)
    ));
    assert!(matches!(
        heap.intersect_mask_demand(correspondence, &[left, right], &mut budget),
        Err(MaskError::LayoutMismatch)
    ));
    let foreign = Heap::new(HeapLimits::default())?;
    assert!(matches!(
        foreign.mask(left, layout),
        Err(MaskError::Heap(HeapError::ForeignHandle))
    ));
    assert_mask(&heap, left, &[0, 64].into_iter().collect())?;
    Ok(())
}

/// Mask slot reuse and changed component strides cannot revive stale versions.
/// ´test:unit:mask-occupation-and-layout-generations´
#[test]
fn mask_occupation_and_layout_generations() -> TestResult {
    let (mut heap, layout, correspondence, mut budget) = fixture(65)?;
    let old = heap.create_mask(layout, &[0, 64], &mut budget)?;
    let changed = heap.create_mask(layout, &[1, 63], &mut budget)?;
    assert_mask(&heap, old, &[0, 64].into_iter().collect())?;
    assert_mask(&heap, changed, &[1, 63].into_iter().collect())?;
    assert_ne!(old, changed);
    heap.retire_mask(old, &mut budget)?;
    let fresh = heap.create_mask(layout, &[0, 64], &mut budget)?;
    assert_eq!(old.key.slot, fresh.key.slot);
    assert!(fresh.key.generation > old.key.generation);
    assert!(matches!(
        heap.mask(old, layout),
        Err(MaskError::Heap(HeapError::StaleHandle))
    ));
    assert!(matches!(
        heap.union_masks(correspondence, old, fresh, &mut budget),
        Err(MaskError::Heap(HeapError::StaleHandle))
    ));
    let next = heap.replace_cell_layout(
        layout,
        Layout::strided(Extent::new(65, 3)?, 195, 0, 3, 1)?,
        &mut budget,
    )?;
    assert_eq!(layout.key.slot, next.key.slot);
    assert!(next.key.generation > layout.key.generation);
    assert!(matches!(
        heap.mask(fresh, layout),
        Err(MaskError::Heap(HeapError::StaleHandle))
    ));
    assert!(matches!(
        heap.mask_correspondence(layout, next, &mut budget),
        Err(MaskError::Heap(HeapError::StaleHandle))
    ));
    let replacement = heap.create_mask(next, &[2], &mut budget)?;
    assert_mask(&heap, replacement, &std::iter::once(2).collect())?;
    heap.retire_mask(fresh, &mut budget)?;
    Ok(())
}

/// Capacity is cells, independent of three-component addressing, and invalid bits never publish.
/// ´test:unit:mask-cell-capacity-and-atomic-refusal´
#[test]
fn mask_cell_capacity_and_atomic_refusal() -> TestResult {
    let (mut heap, layout, _, mut budget) = fixture(65)?;
    let mask = heap.create_mask(layout, &[0, 64, 64], &mut budget)?;
    let view = heap.mask(mask, layout)?;
    assert_eq!(view.components().extent().entries(), 195);
    assert_eq!(view.words().len(), 2);
    assert_eq!(view.words(), &[1, 1]);
    let before = heap.masks.masks.iter().count();
    assert!(matches!(
        heap.create_mask(layout, &[0, 65], &mut budget),
        Err(MaskError::CellOutOfBounds)
    ));
    assert_eq!(heap.masks.masks.iter().count(), before);
    assert_mask(&heap, mask, &[0, 64].into_iter().collect())?;
    Ok(())
}

/// Repeated native retirement reuses bounded metadata while stale handles stay refused.
/// ´test:unit:mask-retirement-bounded-residence´
#[test]
fn mask_retirement_bounded_residence() -> TestResult {
    let (mut heap, layout, correspondence, mut budget) = fixture(65)?;
    let original = heap.create_mask(layout, &[1, 64], &mut budget)?;
    let first = heap.compact_mask(correspondence, original, 5, 2, &mut budget)?;
    let peak = heap.mask_storage_bytes(&mut budget)?;
    heap.retire_mask(first, &mut budget)?;
    heap.retire_cell_layout(first.layout(), &mut budget)?;
    let retired = heap.mask_storage_bytes(&mut budget)?;
    assert!(retired < peak);
    for _ in 0..8 {
        let next = heap.compact_mask(correspondence, original, 5, 2, &mut budget)?;
        assert_eq!(heap.mask_storage_bytes(&mut budget)?, peak);
        assert_ne!(next, first);
        assert_ne!(next.layout(), first.layout());
        heap.retire_cell_layout(next.layout(), &mut budget)?;
        assert!(matches!(
            heap.mask(next, next.layout()),
            Err(MaskError::Heap(HeapError::StaleHandle))
        ));
        heap.retire_mask(next, &mut budget)?;
        assert_eq!(heap.mask_storage_bytes(&mut budget)?, retired);
    }
    assert_mask(&heap, original, &[1, 64].into_iter().collect())?;
    Ok(())
}

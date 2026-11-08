// SPDX-License-Identifier: AGPL-3.0-only
//! Gather controls reconstruct original identity independently of the production bit scan.

use super::{TestResult, assert_mask, fixture, ledger, pattern, publish};
use crate::MaskError;

/// Noncontiguous branches preserve original-index order and independent component associations.
/// ´test:unit:mask-compaction-original-map´
#[test]
fn mask_compaction_original_map() -> TestResult {
    for capacity in [0, 1, 63, 64, 65, 128] {
        for kind in 0..5 {
            let (mut heap, layout, correspondence, mut budget) = fixture(capacity)?;
            let selected = pattern(capacity, kind);
            let branch = publish(&mut heap, layout, &selected, &mut budget)?;
            let compacted =
                heap.compact_mask(correspondence, branch, capacity + 3, capacity, &mut budget)?;
            assert_ne!(compacted.layout(), branch.layout());
            assert_mask(&heap, compacted, &(0..selected.len()).collect())?;
            assert_mask(&heap, branch, &selected)?;
            let view = heap.mask(compacted, compacted.layout())?;
            let gather = view.gather().ok_or("missing gather")?;
            let expected: Vec<_> = selected.iter().copied().collect();
            assert_eq!(gather.indices(), expected);
            assert_eq!(gather.source(), branch);
            assert_eq!(gather.original_axis(), heap.mask(branch, layout)?.axis());
            assert_ne!(gather.original_axis(), view.axis());
            let source: Vec<_> = (0..capacity)
                .map(|cell| [11 * cell + 3, 7 * cell + 5])
                .collect();
            let moved: Vec<_> = gather.indices().iter().map(|&cell| source[cell]).collect();
            let expected_components: Vec<_> = (0..capacity)
                .filter(|cell| selected.contains(cell))
                .map(|cell| [11 * cell + 3, 7 * cell + 5])
                .collect();
            assert_eq!(moved, expected_components);
        }
    }
    Ok(())
}

/// Gather capacity, output capacity and arithmetic refusal leave the original branch readable.
/// ´test:unit:mask-compaction-capacity-and-overflow´
#[test]
fn mask_compaction_capacity_and_overflow() -> TestResult {
    let (mut heap, layout, correspondence, mut budget) = fixture(128)?;
    let branch = heap.create_mask(layout, &[1, 4, 63, 64, 101, 127], &mut budget)?;
    let before = (
        heap.masks.layouts.iter().count(),
        heap.masks.masks.iter().count(),
    );
    assert!(matches!(
        heap.compact_mask(correspondence, branch, 8, 5, &mut budget),
        Err(MaskError::MapCapacity {
            required: 6,
            limit: 5
        })
    ));
    assert!(matches!(
        heap.compact_mask(correspondence, branch, 5, 6, &mut budget),
        Err(MaskError::CompactionCapacity)
    ));
    assert_eq!(
        (
            heap.masks.layouts.iter().count(),
            heap.masks.masks.iter().count()
        ),
        before
    );
    assert!(matches!(
        super::super::storage::add_population(usize::MAX, 1),
        Err(MaskError::CountOverflow)
    ));
    assert!(matches!(
        super::super::memory::bytes::<u64>(usize::MAX),
        Err(MaskError::ByteOverflow)
    ));
    assert_mask(
        &heap,
        branch,
        &[1, 4, 63, 64, 101, 127].into_iter().collect(),
    )?;
    let compacted = heap.compact_mask(correspondence, branch, 6, 6, &mut budget)?;
    assert_mask(&heap, compacted, &(0..6).collect())?;
    Ok(())
}

/// Publication rejects duplicate, omitted, substituted or incompletely moved cells.
/// ´test:unit:mask-compaction-publication-validation´
#[test]
fn mask_compaction_publication_validation() -> TestResult {
    use super::super::compaction::validate_movement;
    let source = [0b1_0010_u64, 1_u64];
    let mut budget = ledger()?;
    validate_movement(&source, &[1, 4, 64], &[0b111], 3, &mut budget)?;
    for map in [
        &[1, 1, 64][..],
        &[1, 4][..],
        &[1, 4, 63][..],
        &[64, 4, 1][..],
    ] {
        assert!(matches!(
            validate_movement(&source, map, &[0b111], 3, &mut budget),
            Err(MaskError::IncompleteMovement)
        ));
    }
    for destination in [&[0b11][..], &[0b1111][..], &[][..]] {
        assert!(matches!(
            validate_movement(&source, &[1, 4, 64], destination, 3, &mut budget),
            Err(MaskError::IncompleteMovement)
        ));
    }
    Ok(())
}

/// Branch algebra and nested compaction retain the original axis after intermediate retirement.
/// ´test:unit:mask-compaction-shared-original-associations´
#[test]
fn mask_compaction_shared_original_associations() -> TestResult {
    let (mut heap, layout, correspondence, mut budget) = fixture(128)?;
    let original = heap.create_mask(layout, &[1, 7, 63, 64, 101], &mut budget)?;
    let axis = heap.mask(original, layout)?.axis();
    let first = heap.compact_mask(correspondence, original, 8, 5, &mut budget)?;
    let first_correspondence =
        heap.mask_correspondence(first.layout(), first.layout(), &mut budget)?;
    let selected = heap.create_mask(first.layout(), &[1, 3, 4], &mut budget)?;
    let branch = heap.intersect_masks(first_correspondence, first, selected, &mut budget)?;
    assert_eq!(
        heap.mask(branch, branch.layout())?
            .gather()
            .ok_or("lost branch map")?
            .indices(),
        &[1, 7, 63, 64, 101]
    );
    assert!(matches!(
        heap.create_mask(first.layout(), &[5], &mut budget),
        Err(MaskError::UnmappedCell)
    ));
    let second = heap.compact_mask(first_correspondence, branch, 65, 3, &mut budget)?;
    let second_view = heap.mask(second, second.layout())?;
    let map = second_view.gather().ok_or("missing composed map")?;
    assert_eq!(map.indices(), &[7, 64, 101]);
    assert_eq!(map.parent(), branch);
    assert_eq!(map.source(), original);
    assert_eq!(map.original_axis(), axis);
    heap.retire_mask(first, &mut budget)?;
    heap.retire_mask(selected, &mut budget)?;
    heap.retire_mask(branch, &mut budget)?;
    heap.retire_cell_layout(first.layout(), &mut budget)?;
    assert_eq!(
        heap.mask(second, second.layout())?
            .gather()
            .ok_or("lost original map")?
            .indices(),
        &[7, 64, 101]
    );
    assert_mask(&heap, second, &(0..3).collect())?;
    let replacement = heap.replace_cell_layout(
        second.layout(),
        crate::Layout::strided(crate::Extent::new(65, 3)?, 195, 0, 3, 1)?,
        &mut budget,
    )?;
    let retained = heap.create_mask(replacement, &[0, 2], &mut budget)?;
    assert_eq!(
        heap.mask(retained, replacement)?
            .gather()
            .ok_or("lost replacement map")?
            .indices(),
        &[7, 64, 101]
    );
    assert_mask(&heap, retained, &[0, 2].into_iter().collect())?;
    Ok(())
}

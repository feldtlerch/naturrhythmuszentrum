// SPDX-License-Identifier: AGPL-3.0-only
//! Set-theoretic controls do not derive their expectations from native word operations.

use super::{TestResult, assert_mask, fixture, pattern, publish};
use crate::{MaskError, MaskPartition};
use std::collections::BTreeSet;

/// Generated empty, single, partial, sparse and full sets preserve all boundary padding.
/// ´test:unit:mask-packed-storage-and-algebra´
#[test]
fn mask_packed_storage_and_algebra() -> TestResult {
    for capacity in [0, 1, 7, 63, 64, 65, 128] {
        for left_kind in 0..5 {
            for right_kind in 0..5 {
                let (mut heap, layout, correspondence, mut budget) = fixture(capacity)?;
                let left_set = pattern(capacity, left_kind);
                let right_set = pattern(capacity, right_kind);
                let left = publish(&mut heap, layout, &left_set, &mut budget)?;
                let right = publish(&mut heap, layout, &right_set, &mut budget)?;
                assert_mask(&heap, left, &left_set)?;
                assert_mask(&heap, right, &right_set)?;
                assert_eq!(heap.mask_population(left, &mut budget)?, left_set.len());
                let intersection =
                    heap.intersect_masks(correspondence, left, right, &mut budget)?;
                let union = heap.union_masks(correspondence, left, right, &mut budget)?;
                let complement = heap.complement_mask(correspondence, left, right, &mut budget)?;
                assert_mask(
                    &heap,
                    intersection,
                    &left_set.intersection(&right_set).copied().collect(),
                )?;
                assert_mask(&heap, union, &left_set.union(&right_set).copied().collect())?;
                assert_mask(
                    &heap,
                    complement,
                    &left_set.difference(&right_set).copied().collect(),
                )?;
                assert_mask(&heap, left, &left_set)?;
                assert_mask(&heap, right, &right_set)?;
                assert_ne!(left, intersection);
            }
        }
    }
    Ok(())
}

/// Every decision role is disjoint and together covers exactly the participating filled set.
/// ´test:unit:mask-decision-partition´
#[test]
fn mask_decision_partition() -> TestResult {
    let (mut heap, layout, correspondence, mut budget) = fixture(128)?;
    let filled = heap.create_mask(layout, &[1, 2, 4, 7, 64], &mut budget)?;
    let first = heap.create_mask(layout, &[1], &mut budget)?;
    let second = heap.create_mask(layout, &[2, 4], &mut budget)?;
    let undecided = heap.create_mask(layout, &[7], &mut budget)?;
    let refusal = heap.create_mask(layout, &[64], &mut budget)?;
    let overlap = heap.create_mask(layout, &[4], &mut budget)?;
    let outside = heap.create_mask(layout, &[65], &mut budget)?;
    let selected = [first, second];
    let undecided_masks = [undecided];
    let refusal_masks = [refusal];
    let valid = MaskPartition {
        selected: &selected,
        undecided: &undecided_masks,
        refusal: &refusal_masks,
    };
    heap.check_mask_partition(correspondence, filled, valid, &mut budget)?;
    assert!(matches!(
        heap.check_mask_partition(
            correspondence,
            filled,
            MaskPartition {
                refusal: &[refusal, overlap],
                ..valid
            },
            &mut budget
        ),
        Err(MaskError::PartitionOverlap)
    ));
    assert!(matches!(
        heap.check_mask_partition(
            correspondence,
            filled,
            MaskPartition {
                refusal: &[],
                ..valid
            },
            &mut budget
        ),
        Err(MaskError::PartitionCoverage)
    ));
    assert!(matches!(
        heap.check_mask_partition(
            correspondence,
            filled,
            MaskPartition {
                refusal: &[refusal, outside],
                ..valid
            },
            &mut budget
        ),
        Err(MaskError::PartitionCoverage)
    ));
    let empty = heap.create_mask(layout, &[], &mut budget)?;
    heap.check_mask_partition(
        correspondence,
        empty,
        MaskPartition {
            selected: &[],
            undecided: &[],
            refusal: &[],
        },
        &mut budget,
    )?;
    Ok(())
}

/// One missing required input cell remains unavailable, including across a word boundary.
/// ´test:unit:mask-required-input-demand´
#[test]
fn mask_required_input_demand() -> TestResult {
    for capacity in [1, 63, 64, 65, 128] {
        let (mut heap, layout, correspondence, mut budget) = fixture(capacity)?;
        let all = (0..capacity).collect::<BTreeSet<_>>();
        let available = (0..capacity - 1).collect::<BTreeSet<_>>();
        let first = publish(&mut heap, layout, &all, &mut budget)?;
        let second = publish(&mut heap, layout, &available, &mut budget)?;
        let third = publish(&mut heap, layout, &all, &mut budget)?;
        let result =
            heap.intersect_mask_demand(correspondence, &[first, second, third], &mut budget)?;
        assert_mask(&heap, result, &available)?;
        assert_eq!(heap.mask_population(result, &mut budget)?, capacity - 1);
        assert!(matches!(
            heap.intersect_mask_demand(correspondence, &[], &mut budget),
            Err(MaskError::MissingInputs)
        ));
    }
    Ok(())
}

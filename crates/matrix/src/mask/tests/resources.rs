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

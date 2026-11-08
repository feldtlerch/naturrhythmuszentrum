// SPDX-License-Identifier: AGPL-3.0-only
//! Existing checked payload reservations cover simultaneous old and new native owners.

use super::{
    AT, MaskError, charge, count,
    storage::{LayoutRecord, MaskRecord, Native},
};
use crate::heap::identity::Key;
use nrz_bindings::{Budget, readings::algebra::limits::BudgetKind};

/// Pre-allocation ceilings include table capacity, payload and temporary publication vectors.
#[derive(Clone, Copy, Debug)]
pub(super) struct Reservation {
    /// Newly retained owners and actual table capacity increases.
    retained: usize,
    /// Per-call retained owners, replacement peak and temporary publication vectors.
    staged: usize,
}

impl Reservation {
    /// Plan exact growth while all previous allocations remain alive.
    pub(super) fn new(
        heap: &crate::Heap,
        layout_bytes: Option<usize>,
        words: Option<usize>,
        map_entries: usize,
        retired: [&[Key]; 2],
        budget: &mut Budget,
    ) -> Result<Self, MaskError> {
        let store = &heap.masks;
        let layouts = usize::from(layout_bytes.is_some());
        let masks = usize::from(words.is_some());
        store.owned_bytes(budget)?;
        let layout_growth = store.layouts.growth_bytes_replacing(layouts, retired[0])?;
        let mask_growth = store.masks.growth_bytes_replacing(masks, retired[1])?;
        let mut retained = add(layout_growth.0, mask_growth.0)?;
        retained = add(retained, layout_bytes.unwrap_or(0))?;
        retained = add(retained, bytes::<u64>(words.unwrap_or(0))?)?;
        retained = add(retained, bytes::<usize>(map_entries)?)?;
        retained = add(retained, layouts * crate::loan::rc_bytes::<LayoutRecord>())?;
        retained = add(retained, masks * crate::loan::rc_bytes::<MaskRecord>())?;
        let keys = add(
            add(layouts, masks)?,
            add(retired[0].len(), retired[1].len())?,
        )?;
        let old_tables = add(layout_growth.1, mask_growth.1)?;
        let mut scratch = add(
            add(old_tables, old_tables)?,
            add(
                bytes::<Key>(keys)?,
                add(
                    bytes::<Native<LayoutRecord>>(layouts)?,
                    bytes::<Native<MaskRecord>>(masks)?,
                )?,
            )?,
        )?;
        if layouts != 0 {
            // Only replaced owners coexist with new payloads; pure retirement stages only keys.
            // This preserves the per-call scope of ``rule:nrz:matrix-loans-legacy-preflight``.
            for &key in retired[0] {
                scratch = add(scratch, store.layouts.get(key)?.owner()?.owned_bytes()?)?;
            }
        }
        let plan = crate::heap::resources::ResourcePlan {
            retained,
            scratch,
            ..crate::heap::resources::ResourcePlan::default()
        };
        heap.preflight_resources(
            plan,
            crate::heap::resources::ResourcePolicy::from_budget(budget),
        )
        .map_err(|error| {
            if let crate::HeapError::ResourceBytes { staged, used, .. } = error {
                let kind = if staged {
                    BudgetKind::StagedPayloadBytes
                } else {
                    BudgetKind::OutputBytes
                };
                if let Err(fault) = budget.check(kind, used, AT) {
                    return MaskError::Budget(fault);
                }
            }
            MaskError::Heap(error)
        })?;
        Ok(Self {
            retained: plan.retained,
            staged: add(plan.retained, plan.scratch)?,
        })
    }

    /// Limits are restored on success and typed refusal; consumed Work is never refunded.
    pub(super) fn run<T>(
        self,
        budget: &mut Budget,
        operation: impl FnOnce(&mut Budget) -> Result<T, MaskError>,
    ) -> Result<T, MaskError> {
        budget.check(BudgetKind::OutputBytes, count(self.retained)?, AT)?;
        budget.with_retained_payload(count(self.staged)?, AT, operation)
    }
}

/// Explicit byte overflow never masquerades as a small reservation.
pub(super) fn add(left: usize, right: usize) -> Result<usize, MaskError> {
    left.checked_add(right).ok_or(MaskError::ByteOverflow)
}

/// Native payload extent, distinct from its logical population.
pub(super) fn bytes<T>(length: usize) -> Result<usize, MaskError> {
    length
        .checked_mul(size_of::<T>())
        .ok_or(MaskError::ByteOverflow)
}

/// An exact reservation precedes this fallible allocator; no vector grows afterward.
pub(super) fn vector<T>(capacity: usize) -> Result<Vec<T>, MaskError> {
    bytes::<T>(capacity)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| crate::HeapError::Capacity)?;
    if result.capacity() != capacity {
        return Err(crate::HeapError::Capacity.into());
    }
    Ok(result)
}

/// Initialize every addressable word inside the original allocation receipt.
pub(super) fn zeroed(words: usize, budget: &mut Budget) -> Result<Vec<u64>, MaskError> {
    budget.with_work_phase(super::FAMILY, "mask allocation", |budget| {
        charge(budget, words)?;
        let mut result = vector(words)?;
        result.resize(words, 0);
        Ok(result)
    })
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Existing checked payload reservations cover simultaneous old and new native owners.

use super::{
    AT, MaskError, charge, count,
    storage::{LayoutRecord, MaskRecord, Store},
};
use crate::heap::identity::Key;
use nrz_bindings::{Budget, readings::algebra::limits::BudgetKind};

/// Pre-allocation ceilings include table capacity, payload and temporary publication vectors.
#[derive(Clone, Copy, Debug)]
pub(super) struct Reservation {
    /// Simultaneously retained owners, including vacant table capacity.
    retained: usize,
    /// Retained owners plus prepared keys, retired keys and unpublished owner records.
    staged: usize,
}

impl Reservation {
    /// Plan exact growth while all previous allocations remain alive.
    pub(super) fn new(
        store: &Store,
        layout_bytes: Option<usize>,
        words: Option<usize>,
        map_entries: usize,
        retired: [usize; 2],
        budget: &mut Budget,
    ) -> Result<Self, MaskError> {
        let layouts = usize::from(layout_bytes.is_some());
        let masks = usize::from(words.is_some());
        let mut retained = store.owned_bytes(budget)?;
        retained = add(
            retained,
            store
                .layouts
                .reserved_bytes(layouts)?
                .checked_sub(store.layouts.owned_bytes()?)
                .ok_or(MaskError::ByteOverflow)?,
        )?;
        retained = add(
            retained,
            store
                .masks
                .reserved_bytes(masks)?
                .checked_sub(store.masks.owned_bytes()?)
                .ok_or(MaskError::ByteOverflow)?,
        )?;
        retained = add(retained, layout_bytes.unwrap_or(0))?;
        retained = add(retained, bytes::<u64>(words.unwrap_or(0))?)?;
        retained = add(retained, bytes::<usize>(map_entries)?)?;
        let keys = add(add(layouts, masks)?, add(retired[0], retired[1])?)?;
        let scratch = add(
            bytes::<Key>(keys)?,
            add(bytes::<LayoutRecord>(layouts)?, bytes::<MaskRecord>(masks)?)?,
        )?;
        Ok(Self {
            retained,
            staged: add(retained, scratch)?,
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

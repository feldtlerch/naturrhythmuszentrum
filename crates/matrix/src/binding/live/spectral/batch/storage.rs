// SPDX-License-Identifier: AGPL-3.0-only
//! The scheduler owns two vectors and fixed fields; shared numerical owners are separate.

use super::{AT, Entry, Header, HeapError, Key, LcSpectralBatch, Row, SpectralError};
use nrz_bindings::{Budget, readings::algebra::limits::BudgetKind};
use nrz_scalar::Scalar;

/// Scoped scheduler inventory, excluding original source groups, exact evidence and child scratch.
#[derive(Clone, Copy, Debug)]
pub struct LcSpectralBatchInventory {
    /// Admitted stable point rows.
    pub points: usize,
    /// Reserved invariant slots; failed and stale entries never free a slot.
    pub binding_capacity: usize,
    /// Typed key header and reserved ordered selected-reference slots.
    pub key_bytes: u64,
    /// Requested vector lengths plus the fixed owner, including inline result/status slots.
    pub logical_payload_bytes: u64,
    /// Actual vector capacities plus the fixed owner; allocator headers are excluded.
    pub observed_capacity_bytes: u64,
}

/// Admit every fixed key, status and result slot before constructing the first key.
pub(super) fn planned<P: Scalar>(
    points: usize,
    bindings: usize,
    budget: &Budget,
) -> Result<LcSpectralBatchInventory, SpectralError> {
    let key_bytes = bytes(bindings, size_of::<Key<P>>(), size_of::<Header>())?;
    let rows = bytes(points, size_of::<Row>(), size_of::<LcSpectralBatch<P>>())?;
    let logical = bytes(bindings, size_of::<Entry<P>>(), 0)?
        .checked_add(rows)
        .ok_or(HeapError::ExtentOverflow)?;
    budget.check(BudgetKind::StagedPayloadBytes, logical, AT)?;
    budget.check(BudgetKind::OutputBytes, logical, AT)?;
    Ok(LcSpectralBatchInventory {
        points,
        binding_capacity: bindings,
        key_bytes,
        logical_payload_bytes: logical,
        observed_capacity_bytes: 0,
    })
}

/// Allocator capacity is reported separately and checked before the owner can escape.
pub(super) fn observed<P: Scalar>(
    rows: &Vec<Row>,
    entries: &Vec<Entry<P>>,
    budget: &Budget,
) -> Result<u64, SpectralError> {
    let row_bytes = bytes(
        rows.capacity(),
        size_of::<Row>(),
        size_of::<LcSpectralBatch<P>>(),
    )?;
    let total = bytes(entries.capacity(), size_of::<Entry<P>>(), 0)?
        .checked_add(row_bytes)
        .ok_or(HeapError::ExtentOverflow)?;
    budget.check(BudgetKind::StagedPayloadBytes, total, AT)?;
    budget.check(BudgetKind::OutputBytes, total, AT)?;
    Ok(total)
}

/// Checked arithmetic precedes every capacity request and key bound comparison.
fn bytes(count: usize, width: usize, fixed: usize) -> Result<u64, SpectralError> {
    let value = count
        .checked_mul(width)
        .and_then(|value| value.checked_add(fixed))
        .ok_or(HeapError::ExtentOverflow)?;
    u64::try_from(value).map_err(|_| HeapError::ExtentOverflow.into())
}

impl<P: Scalar> LcSpectralBatch<P> {
    /// Disjoint new scheduler storage, one schema and the distinct original plan records.
    pub(super) fn resident_bytes(&self) -> Result<u64, SpectralError> {
        let mut bytes = self
            .inventory
            .observed_capacity_bytes
            .checked_add(self.header.schema.retained_payload_bytes())
            .ok_or(HeapError::ExtentOverflow)?;
        for entry in &self.entries {
            bytes = bytes
                .checked_add(entry.plan.retained_payload_bytes())
                .ok_or(HeapError::ExtentOverflow)?;
        }
        Ok(bytes)
    }
}

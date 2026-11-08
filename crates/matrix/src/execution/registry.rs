// SPDX-License-Identifier: AGPL-3.0-only
//! Successful publication transfers the lease; retirement drops only that execution's share.

use super::lease::Share;
use super::{ExecutionInventory, Lease};
use crate::{ExecutionId, Heap, HeapError, OperationError, RecordId};

/// Immutable history and independently releasable numerical resource ownership.
#[derive(Debug)]
struct Entry {
    /// Exact heap-scoped operation record that completed this execution.
    record: RecordId,
    /// No numerical references survive here after retirement.
    active: Option<Vec<Share>>,
    /// Historical measurements and original tickets remain readable.
    inventory: ExecutionInventory,
}
/// Heap-owned completed executions; publication reserves capacity before its no-fail commit.
#[derive(Debug, Default)]
pub(crate) struct Registry {
    /// Completed records retain immutable observations and independently releasable shares.
    entries: Vec<Entry>,
    /// Original live allocation counters include unpublished escaped factors.
    traces: Vec<super::WeakTrace>,
    /// Active edge-row count keeps inventory cost planning allocation-free and constant time.
    active_shares: usize,
}
impl Registry {
    /// Unpublished escaped factors participate in the heap's zero-occupied predicate.
    pub(crate) fn has_payload(&self) -> bool {
        self.traces
            .iter()
            .fold(false, |present, trace| trace.has_payload() | present)
    }
    /// Every visited weak row, including expired rows, belongs to common inventory Work.
    pub(crate) const fn scan_work(&self) -> usize {
        self.traces.len() + self.entries.len() + self.active_shares
    }
    /// Residence additionally inspects each weak counter for a live numerical owner.
    pub(crate) const fn trace_work(&self) -> usize {
        self.traces.len()
    }
    /// Unique original counters and retained registry buffers, never historical payload totals.
    pub(crate) fn owned_bytes(&self) -> Result<usize, HeapError> {
        let add = crate::heap::resources::add;
        let mut bytes = self
            .entries
            .capacity()
            .checked_mul(size_of::<Entry>())
            .ok_or(HeapError::Capacity)?;
        bytes = add(
            bytes,
            self.traces
                .capacity()
                .checked_mul(size_of::<super::WeakTrace>())
                .ok_or(HeapError::Capacity)?,
        )?;
        for trace in &self.traces {
            bytes = add(bytes, trace.owned_bytes()?)?;
        }
        for entry in &self.entries {
            bytes = add(
                bytes,
                entry
                    .inventory
                    .factors
                    .capacity()
                    .checked_mul(size_of::<super::SharedFactorAllocation>())
                    .ok_or(HeapError::Capacity)?,
            )?;
            if let Some(shares) = &entry.active {
                bytes = add(
                    bytes,
                    shares
                        .capacity()
                        .checked_mul(size_of::<Share>())
                        .ok_or(HeapError::Capacity)?,
                )?;
            }
        }
        Ok(bytes)
    }
    /// Complete new counter and exact registry growth precede the execution's first allocation.
    pub(crate) fn admission_plan(&self) -> Result<crate::heap::resources::ResourcePlan, HeapError> {
        let reused = self.traces.iter().any(super::WeakTrace::expired);
        let trace_count = self
            .traces
            .len()
            .checked_add(usize::from(!reused))
            .ok_or(HeapError::Capacity)?;
        let entry_count = self
            .entries
            .len()
            .checked_add(1)
            .ok_or(HeapError::Capacity)?;
        // A new counter is allocated before registration can destroy an expired weak header.
        let mut retained = super::Trace::metadata_bytes();
        let mut scratch = if reused {
            super::Trace::metadata_bytes()
        } else {
            0
        };
        for (old, new, width) in [
            (
                self.traces.capacity(),
                trace_count,
                size_of::<super::WeakTrace>(),
            ),
            (self.entries.capacity(), entry_count, size_of::<Entry>()),
        ] {
            let new = crate::heap::slots::growth_capacity::<Entry>(old, new)?;
            if new > old {
                retained = crate::heap::resources::add(
                    retained,
                    (new - old).checked_mul(width).ok_or(HeapError::Capacity)?,
                )?;
                scratch = crate::heap::resources::add(
                    scratch,
                    old.checked_mul(width)
                        .and_then(|bytes| bytes.checked_mul(2))
                        .ok_or(HeapError::Capacity)?,
                )?;
            }
        }
        Ok(crate::heap::resources::ResourcePlan {
            retained,
            scratch,
            ..Default::default()
        })
    }
    /// Consume previously admitted backing capacity and install a metadata-only weak witness.
    pub(crate) fn register(&mut self, trace: &super::Trace) -> Result<(), OperationError> {
        grow(&mut self.entries, 1)?;
        if let Some(entry) = self.traces.iter_mut().find(|entry| entry.expired()) {
            *entry = trace.downgrade();
        } else {
            grow(&mut self.traces, 1)?;
            self.traces.push(trace.downgrade());
        }
        Ok(())
    }
    /// Exact aligned inline registry entry charged to this execution's metadata budget.
    pub(crate) const fn entry_bytes() -> usize {
        size_of::<Entry>()
    }
    /// Prepare all allocation and exact ticket checks before publishing any output.
    pub(crate) fn prepare<P: nrz_scalar::Scalar>(
        &mut self,
        lease: &Lease,
        candidate: &crate::Candidate<P>,
    ) -> Result<ExecutionInventory, OperationError> {
        lease.validate(candidate)?;
        let inventory = lease.snapshot()?;
        self.active_shares
            .checked_add(inventory.factors.len())
            .ok_or(OperationError::ResourceLimit)?;
        self.entries
            .try_reserve(1)
            .map_err(|_| OperationError::ResourceLimit)?;
        Ok(inventory)
    }
    /// Commit only prepared metadata and move the existing numerical shares without cloning them.
    pub(crate) fn publish(
        &mut self,
        record: RecordId,
        lease: Lease,
        inventory: ExecutionInventory,
    ) {
        let shares = lease.into_shares();
        self.active_shares += shares.len();
        self.entries.push(Entry {
            record,
            active: Some(shares),
            inventory,
        });
    }
    /// No fallible work follows removal of this one execution's ownership share.
    fn retire(&mut self, record: RecordId, execution: ExecutionId) -> Result<(), HeapError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.record == record)
            .ok_or(HeapError::StaleEvidence)?;
        if entry.inventory.execution() != execution {
            return Err(HeapError::StaleEvidence);
        }
        let active = entry.active.take().ok_or(HeapError::StaleHandle)?;
        self.active_shares -= active.len();
        drop(active);
        Ok(())
    }
}
impl Heap {
    /// Retire the completed execution's private resource shares in one call.
    ///
    /// Independent external factors and other completed consumers retain their own shares.
    /// Published value regions and immutable operation history have separate lifetimes.
    /// # Errors
    /// Refuses a foreign record, an unknown execution or repeated retirement without mutation.
    pub fn retire_execution(&mut self, record: RecordId) -> Result<(), HeapError> {
        let execution = self.historical_record(record)?.execution();
        self.executions.retire(record, execution)
    }
    /// Inspect immutable private allocation observations after publication or retirement.
    /// # Errors
    /// Refuses a foreign or unknown completed operation record.
    pub fn execution_inventory(&self, record: RecordId) -> Result<&ExecutionInventory, HeapError> {
        self.historical_record(record)?;
        self.executions
            .entries
            .iter()
            .find(|entry| entry.record == record)
            .map(|entry| &entry.inventory)
            .ok_or(HeapError::StaleEvidence)
    }
}

/// Exact replacement retains the old table until the admitted new buffer is complete.
fn grow<T>(values: &mut Vec<T>, additional: usize) -> Result<(), OperationError> {
    let count = values
        .len()
        .checked_add(additional)
        .ok_or(OperationError::ResourceLimit)?;
    if count > values.capacity() {
        let count = crate::heap::slots::growth_capacity::<T>(values.capacity(), count)?;
        let mut replacement = crate::binding::buffer(count)?;
        if replacement.capacity() != count {
            return Err(OperationError::ResourceLimit);
        }
        replacement.append(values);
        *values = replacement;
    }
    Ok(())
}

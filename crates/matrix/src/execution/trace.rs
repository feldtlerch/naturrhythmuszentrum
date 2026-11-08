// SPDX-License-Identifier: AGPL-3.0-only
//! One counter observes overlapping scoped allocation lifetimes in an execution.

use crate::{ExecutionId, OperationError};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

/// Counters retain no numerical owner and cannot extend a resource lifetime.
#[derive(Debug)]
struct Counters {
    /// Original validated execution identity.
    execution: ExecutionId,
    /// Actual currently owned observable vector capacities.
    live: Cell<usize>,
    /// Live opaque allowances and measured owner bytes outside the observed vector schedules.
    residence: Cell<usize>,
    /// Maximum observed by this common counter, never a sum of scoped peaks.
    peak: Cell<usize>,
    /// Sum of actual capacities observed over successful allocation events.
    bytes: Cell<usize>,
    /// Number of successful observed allocation requests.
    allocations: Cell<usize>,
}
/// Shared event accounting has the same inline width as the execution identity it replaces.
#[repr(transparent)]
#[derive(Clone, Debug)]
pub(crate) struct Trace(Rc<Counters>);
impl Trace {
    /// Establish one common lifetime counter before the execution's first private allocation.
    pub(crate) fn new(execution: ExecutionId) -> Self {
        Self(Rc::new(Counters {
            execution,
            live: Cell::new(0),
            residence: Cell::new(0),
            peak: Cell::new(0),
            bytes: Cell::new(0),
            allocations: Cell::new(0),
        }))
    }
    /// Register metadata without retaining any numerical owner.
    pub(crate) fn downgrade(&self) -> WeakTrace {
        WeakTrace(Rc::downgrade(&self.0))
    }
    /// Retain bytes once for an owner lifetime, independently of observed allocation statistics.
    pub(crate) fn retain_residence(&self, bytes: usize) -> Result<(), OperationError> {
        let retained = self
            .0
            .residence
            .get()
            .checked_add(bytes)
            .ok_or(OperationError::ResourceLimit)?;
        self.0.residence.set(retained);
        Ok(())
    }
    /// Release the original debit after its final owner ends.
    pub(crate) fn release_residence(&self, bytes: usize) {
        self.0.residence.set(self.0.residence.get() - bytes);
    }
    /// Stable ticket origin, independent of shared resource or buffer addresses.
    pub(crate) fn execution(&self) -> ExecutionId {
        self.0.execution
    }
    /// Record a new actual capacity before another covered allocation can begin.
    pub(crate) fn allocated(&self, bytes: usize) -> Result<(), OperationError> {
        let live = self
            .0
            .live
            .get()
            .checked_add(bytes)
            .ok_or(OperationError::ResourceLimit)?;
        let total = self
            .0
            .bytes
            .get()
            .checked_add(bytes)
            .ok_or(OperationError::ResourceLimit)?;
        let count = self
            .0
            .allocations
            .get()
            .checked_add(1)
            .ok_or(OperationError::ResourceLimit)?;
        self.0.live.set(live);
        self.0.bytes.set(total);
        self.0.allocations.set(count);
        self.0.peak.set(self.0.peak.get().max(live));
        Ok(())
    }
    /// An original permit reports release once, after recording a successful allocation.
    pub(crate) fn released(&self, bytes: usize) {
        self.0.live.set(self.0.live.get() - bytes);
    }
    /// Freeze common-lifetime observations for immutable publication history.
    pub(crate) fn observations(&self) -> [usize; 3] {
        [
            self.0.allocations.get(),
            self.0.bytes.get(),
            self.0.peak.get(),
        ]
    }
    /// Actual aligned counter allocation, including its reference-count header.
    pub(crate) const fn metadata_bytes() -> usize {
        size_of::<RcLayout>()
    }
}
/// Reference-counted allocation header and aligned event counters.
#[repr(C)]
struct RcLayout {
    /// Strong reference count.
    strong: usize,
    /// Weak reference count.
    weak: usize,
    /// Counter payload, with no numerical resource references.
    value: Counters,
}

/// Private weak counter index never grants numerical access or a public upgrade route.
#[derive(Debug)]
pub(crate) struct WeakTrace(Weak<Counters>);
impl WeakTrace {
    /// Expired metadata rows can be reused before extending the index.
    pub(crate) fn expired(&self) -> bool {
        self.0.strong_count() == 0
    }
    /// Historical counter metadata alone does not retain numerical ownership.
    pub(crate) fn has_payload(&self) -> bool {
        self.0
            .upgrade()
            .is_some_and(|owner| owner.live.get() != 0 || owner.residence.get() != 0)
    }
    /// A weak row keeps its counter header allocated even after its last strong owner drops.
    pub(crate) fn owned_bytes(&self) -> Result<usize, crate::HeapError> {
        let payload = self.0.upgrade().map_or(Ok(0), |owner| {
            owner
                .live
                .get()
                .checked_add(owner.residence.get())
                .ok_or(crate::HeapError::Capacity)
        })?;
        payload
            .checked_add(Trace::metadata_bytes())
            .ok_or(crate::HeapError::Capacity)
    }
}

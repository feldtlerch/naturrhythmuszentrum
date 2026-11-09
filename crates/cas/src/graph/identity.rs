// SPDX-License-Identifier: AGPL-3.0-only
//! Token-owned serial reservations survive every graph rollback and test-store copy.

use std::sync::atomic::{AtomicU64, Ordering};

use super::VertexKind;
use crate::budget::BudgetKind;
use crate::error::{Fault, Location};

/// One high-water mark for each stable vertex-kind tag, separate from retained occupancy.
#[derive(Debug)]
pub(super) struct Identities {
    /// Exclusive allocation bounds; a discarded reservation is never returned.
    next: [AtomicU64; 42],
}

impl Default for Identities {
    fn default() -> Self {
        Self {
            next: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
}

impl Identities {
    /// Reserve a complete suffix before publication, refusing overflow without changing it.
    /// Relaxed ordering suffices: only uniqueness is shared, never graph publication.
    pub(super) fn reserve(&self, kind: VertexKind, count: u64, at: Location) -> Result<u64, Fault> {
        self.next
            .get(usize::from(kind.tag()))
            .ok_or_else(|| Fault::capacity(BudgetKind::Vertices, at))?
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(count)
            })
            .map_err(|_| Fault::capacity(BudgetKind::Vertices, at))
    }
}

#[cfg(test)]
mod tests;

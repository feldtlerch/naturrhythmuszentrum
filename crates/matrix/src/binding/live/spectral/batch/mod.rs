// SPDX-License-Identifier: AGPL-3.0-only
//! A closed autonomous node has one live producer per admitted original input key.

mod admission;
mod driven;
pub use driven::{
    LcDrivenContinuations, LcDrivenError, LcDrivenInventory, LcDrivenLimits, LcDrivenPoint,
    LcDrivenStatistics, LcDrivenStatus, LcDrivenStratum,
};
mod coefficients;
use coefficients::Coefficients;
mod execution;
mod storage;
pub use storage::LcSpectralBatchInventory;

use super::{Heap, HeapError, LcSpectralEvaluation, LcSpectralSchema, LcSpectrum, SpectralError};
use crate::ReadingBinding;
use nrz_bindings::{
    Budget,
    readings::algebra::{Location, limits::BudgetUsage},
};
use nrz_scalar::{ArithmeticContext, Precision, Scalar};

/// Accounting location for this closed scheduler's own lookup and storage work.
const AT: Location = Location::new("LC spectral batch");

/// Explicit finite ceilings; there is no implicit eviction, retry or default policy.
#[derive(Clone, Copy, Debug)]
pub struct LcSpectralBatchLimits {
    /// Maximum retained point rows, each with its original caller key.
    pub points: usize,
    /// Maximum distinct invariant keys, including unsuccessful and stale entries.
    pub bindings: usize,
    /// Maximum bytes in the typed common header and retained selected-input keys.
    pub key_bytes: u64,
}

/// Historical node state; current numerical access separately revalidates every dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcSpectralStatus {
    /// The finite node has not yet been attempted.
    Pending,
    /// The complete autonomous evaluation succeeded.
    Successful,
    /// Original live binding authentication refused.
    BindingRefused,
    /// The actual input family was proved nonpositive.
    DomainUnavailable,
    /// Arithmetic or enclosure separation failed without a successful numerical result.
    NumericallyFailed,
    /// This node exhausted a common caller allowance.
    ResourceRefused,
    /// An earlier resource refusal blocked this unexecuted node.
    Blocked,
}

/// Bounded autonomous spectral schedule with stable rows and insertion-ordered invariant keys.
///
/// The closed node depends on the original semantic schema, scalar/arithmetic policy,
/// and both complete L/C bindings. It has no point-varying inputs or decision successors.
/// Rows with the same authenticated key share the original complete evaluation. This
/// owner does not publish heap output symbols or execute driven current/phase branches.
#[derive(Debug)]
pub struct LcSpectralBatch<P: Scalar> {
    /// Common typed key fields retain their original immutable authority.
    header: Header,
    /// Stable point keys reference complete invariant producers.
    rows: Vec<Row>,
    /// No insertion or eviction is possible after preparation.
    entries: Vec<Entry<P>>,
    /// Scoped logical and observed capacity accounting for this owner only.
    inventory: LcSpectralBatchInventory,
    /// One source-free numerical producer precedes every binding-wide root.
    coefficients: Coefficients<P>,
    /// One finite execution attempt; refusal cannot replenish it.
    attempted: bool,
    /// Complete caller snapshots around execution, including failed children.
    usage: Option<[BudgetUsage; 2]>,
}

/// Node kind and revision are fixed by this closed Rust type, not a portable wire schema.
#[derive(Debug)]
struct Header {
    /// Authenticated original exact law and literal policy.
    schema: LcSpectralSchema,
    /// The heap's original numerical occupation namespace.
    scope: u64,
    /// Scalar precision cannot be inferred from equal centers.
    precision: Precision,
    /// Original outward arithmetic theorem revision.
    revision: u32,
}

/// Retained selected declarations abbreviate maps and complete source/companion snapshots.
#[derive(Debug)]
struct Key<P: Scalar> {
    /// Original L/C order is significant.
    inputs: [ReadingBinding<P>; 2],
}

/// A row contains no copied numerical result or duplicated child resource receipt.
#[derive(Debug)]
struct Row {
    /// Stable caller identity, distinct from the producer ordinal.
    point: u64,
    /// Insertion-ordered invariant node for this point.
    binding: usize,
}

/// Fixed status/result slots are allocated before numerical execution.
#[derive(Debug)]
struct Entry<P: Scalar> {
    /// Typed immutable selection references, compared in declared L/C order.
    key: Key<P>,
    /// First admitted original plan is the producer consumed by later adapters.
    plan: LcSpectrum<P>,
    /// Historical outcome of the one permitted attempt.
    status: LcSpectralStatus,
    /// Complete evaluation exists only after success, with no placeholder values.
    result: Option<LcSpectralEvaluation<P>>,
    /// Original bounded failure diagnostic, never an invented numerical result.
    failure: Option<SpectralError>,
    /// Earlier common-resource failure explains an unexecuted suffix.
    blocked_by: Option<usize>,
}

impl<P: Scalar> LcSpectralBatch<P> {
    /// Original caller keys in their admitted order.
    pub fn point_keys(&self) -> impl ExactSizeIterator<Item = u64> + '_ {
        self.rows.iter().map(|row| row.point)
    }

    /// Historical status of one original row; no live value is exposed here.
    #[must_use]
    pub fn status(&self, row: usize) -> Option<LcSpectralStatus> {
        self.rows
            .get(row)
            .map(|row| self.entries[row.binding].status)
    }

    /// Stable original producer ordinal, shared only by compatible rows.
    #[must_use]
    pub fn binding_index(&self, row: usize) -> Option<usize> {
        self.rows.get(row).map(|row| row.binding)
    }

    /// Original failure, including the earlier cause of a resource-blocked suffix.
    #[must_use]
    pub fn failure(&self, row: usize) -> Option<&SpectralError> {
        let entry = self.entries.get(self.rows.get(row)?.binding)?;
        let entry = entry.blocked_by.map_or(entry, |index| &self.entries[index]);
        entry
            .failure
            .as_ref()
            .or(self.coefficients.failure.as_ref())
    }

    /// Historical outcome of the single source-free coefficient producer.
    #[must_use]
    pub const fn coefficient_status(&self) -> LcSpectralStatus {
        self.coefficients.status
    }

    /// Original coefficient execution receipt, shared by every compatible binding.
    #[must_use]
    pub const fn coefficient_usage(&self) -> Option<[BudgetUsage; 2]> {
        self.coefficients.usage
    }

    /// Distinct retained keys, including unsuccessful or subsequently stale nodes.
    #[must_use]
    pub const fn bindings(&self) -> usize {
        self.entries.len()
    }

    /// New scheduler ownership only, excluding shared schemas and child allocations.
    #[must_use]
    pub const fn inventory(&self) -> &LcSpectralBatchInventory {
        &self.inventory
    }

    /// Complete attempted prefix on the caller ledger; preparation has its own caller charges.
    #[must_use]
    pub const fn usage(&self) -> Option<[BudgetUsage; 2]> {
        self.usage
    }
}

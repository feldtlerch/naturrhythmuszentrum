// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded later strata consume original fixed impedance evidence and preserve every point.

mod access;
mod admission;
mod execution;
mod failure;
mod statistics;
pub use statistics::LcDrivenStatistics;

use super::LcSpectralBatch;
use crate::{AxisPhasePlan, PhasorSolvePublication, PublishedAxisPhase, PublishedPhasorSolve};
pub use failure::LcDrivenError;
use nrz_bindings::{AxisPhaseImpedance, AxisPhaseOutcome, readings::algebra::limits::BudgetUsage};
use nrz_scalar::Scalar;

/// One complete original fixed-impedance row, declared before any numerical stratum.
#[derive(Debug)]
pub struct LcDrivenPoint<P: Scalar> {
    /// Stable caller identity, matched in order to the autonomous point table.
    point: u64,
    /// Complete original current value and companion slots, including unavailable rows.
    current: PhasorSolvePublication<P>,
    /// Both exact phase alternatives share the original certified pi region.
    phase: AxisPhasePlan<P>,
    /// Original question is authenticated against the actual current equation's impedance.
    impedance: AxisPhaseImpedance,
    /// One declared attempt; no implicit refinement or replacement certificate is permitted.
    enclosure: nrz_bindings::readings::algebra::decision::RationalEnclosure,
}

/// Historical node state; reading a successful value still revalidates its complete closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcDrivenStatus {
    /// The declared stratum has not run.
    Pending,
    /// The node completed; this is not a current-version lease.
    Successful,
    /// Proved zero impedance makes current unavailable, never zero.
    DomainUnavailable,
    /// Phase is undefined at proved zero.
    Undefined,
    /// The original enclosure crosses zero; no branch was chosen.
    Undecided,
    /// Original source, symbol or companion authentication refused.
    BindingRefused,
    /// Numerical separation, arithmetic or solve verification refused.
    NumericallyFailed,
    /// The common caller allowance or an applicable child capacity refused.
    ResourceRefused,
    /// A required decision or earlier common resource refusal prevented execution.
    Blocked,
}

/// The finite declared order; current and phase never execute within decision selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcDrivenStratum {
    /// Existing source-free coefficients and shared autonomous roots.
    Invariant,
    /// Original fixed-impedance sign questions, one attempt per point.
    Decision,
    /// Selected original physical current publications.
    Current,
    /// Selected phase enclosures, still external live attachments.
    Phase,
}

impl LcDrivenStratum {
    /// Stable declared order, independent of the number or outcome of point rows.
    const fn index(self) -> usize {
        match self {
            Self::Invariant => 0,
            Self::Decision => 1,
            Self::Current => 2,
            Self::Phase => 3,
        }
    }
}

/// Explicit ceilings for the new continuation table, excluding shared child owners.
#[derive(Clone, Copy, Debug)]
pub struct LcDrivenLimits {
    /// Maximum stable point count, including all unavailable rows.
    pub points: usize,
    /// Maximum fixed owner and actual row-vector capacity in bytes.
    pub table_bytes: u64,
}

/// New table storage only; shared exact declarations and child allocations are excluded.
#[derive(Clone, Copy, Debug)]
pub struct LcDrivenInventory {
    /// Fixed owner plus admitted point, status and result slots.
    pub logical_payload_bytes: u64,
    /// Fixed owner plus actual vector capacity, excluding allocator overhead.
    pub observed_capacity_bytes: u64,
    /// Complete predeclared point count, including unavailable outputs.
    pub points: usize,
}

/// A finite continuation schedule attached to one original autonomous scheduler.
///
/// This increment consumes already admitted fixed impedance blocks. It does not compute
/// them from L/C, compose shared-voltage covariance, or publish phase values by symbol.
/// Those boundaries are independent of the explicit zero and undecided continuations.
#[derive(Debug)]
pub struct LcDrivenContinuations<P: Scalar> {
    /// Existing autonomous owner is retained once, without copied evaluations.
    invariants: LcSpectralBatch<P>,
    /// Stable slots include every declared outcome and complete current publication.
    rows: Vec<Row<P>>,
    /// Next declared stratum; four means the one finite attempt is complete.
    next: usize,
    /// A resource refusal blocks the remaining attempt without refreshing any ledger.
    stopped: bool,
    /// Historical complete caller receipts, including attempted failures.
    usage: [Option<[BudgetUsage; 2]>; 4],
    /// Only this new fixed owner and row allocation; not a whole-attempt memory claim.
    inventory: LcDrivenInventory,
}

/// Complete fixed outcome slots for one original declared point.
#[derive(Debug)]
struct Row<P: Scalar> {
    /// All inputs, alternatives and output slots were declared before execution.
    plan: LcDrivenPoint<P>,
    /// Decision, current and phase status, respectively.
    status: [LcDrivenStatus; 3],
    /// Archived exact outcome; no numerical authority follows from inspecting it.
    verdict: Option<AxisPhaseOutcome>,
    /// Original live attachment is revalidated before each later continuation.
    selection: Option<PublishedAxisPhase<P>>,
    /// Complete current group exists only after successful atomic publication.
    current: Option<PublishedPhasorSolve<P>>,
    /// Original typed failures, never numerical placeholders.
    failure: [Option<LcDrivenError>; 3],
}

/// New scheduler work has one caller location and never creates a child ledger.
const AT: nrz_bindings::readings::algebra::Location =
    nrz_bindings::readings::algebra::Location::new("LC driven continuations");

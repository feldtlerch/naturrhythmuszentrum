// SPDX-License-Identifier: AGPL-3.0-only
//! Private candidates and consuming publication over a frozen checked binding.
//!
//! # Candidate validation and atomic commit
//!
//! `Bound::evaluate` accepts only its own `NormalizedInputs` bundle and issues one unique candidate token after successful recipe evaluation. A `Candidate` owns unpublished centers and complete companions; it is neither cloneable nor a value handle. Repeated evaluation and foreign view bundles refuse. `Bound::publish` consumes the binding and candidate, independently rechecks every copied bit, finite bound, recipe revision, output extent, reserved identity/version and both directions of every companion target. The final `Outcome<Vec<PublishedBundle<P>>>` exposes all outputs together. No prefix becomes visible when a later output fails.
//!
//! Publication fills the already reserved private slabs and prepares all immutable graph records, source edges and append capacities before changing visible state. The commit only installs prepared storage, schemas, descriptors and graph objects through the existing no-fail primitives. It runs no allocation, caller callback or numerical kernel. Internal bounds target the reserved fresh output identities and versions in that same commit. Existing inputs and their versions remain unchanged on every refusal. Scratch consists of the reserved slab set and candidate payload set, each bounded by `ExecutionLimits`; temporary preparation copies and ordinary allocator aborts remain subject to the heap's documented allocation contract.

mod axis_phase;
pub use axis_phase::{AxisPhasePlan, FrozenAxisPhaseInput, PreparedAxisPhase, PublishedAxisPhase};
mod commit;
mod decision;
pub use decision::{
    FrozenRatioKernel, LiveDecisionCount, LiveDecisionError, PreparedDecision, PublishedDecision,
    RatioKernelEvidence,
};
mod decision_inputs;
pub use decision_inputs::{DecisionInputSnapshot, DecisionSnapshotError, FrozenDecisionInputs};
mod lattice;
pub use lattice::{
    LiveLatticeIndex, LiveLatticeRepresentative, PreparedLatticeDecision, PublishedLatticeDecision,
};
mod lattice_enclosure;
pub use lattice_enclosure::FrozenLatticeEnclosure;
mod lattice_attempt;
mod lattice_inputs;
pub use lattice_inputs::FrozenLatticeInputs;
mod evaluate;
mod fill;
mod retirement;

use crate::{ExecutionId, Extent, ValueStamp};
use nrz_scalar::Scalar;

/// Owned unpublished numerical scratch tied to one unique issued execution token.
///
/// Candidates cannot be cloned, resolved as values, or used as evidence-bearing handles.
#[derive(Debug)]
pub struct Candidate<P: Scalar> {
    /// Binding that granted the normalized inputs.
    pub(crate) execution: ExecutionId,
    /// Unique process-local candidate token, consumed by publication.
    pub(crate) token: u64,
    /// Exact native recipe revision used for evaluation.
    pub(crate) revision: u32,
    /// Complete private outputs in operation order.
    pub(crate) outputs: Vec<Payload<P>>,
    /// Independently established per-output recipe evidence.
    pub(crate) evidence: Vec<crate::RecipeEvidence>,
    /// Complete private statistical parameters, published in the same transaction.
    pub(crate) statistics: Vec<crate::statistics::MomentPayload<P>>,
    /// Ticket authority follows the original candidate arrays until their destruction.
    pub(crate) private: Option<crate::private::Run>,
    /// Declared last so both outer vector backings are destroyed before their debit closes.
    pub(crate) _residence: CandidateResidence,
}

/// One non-cloneable debit follows the candidate's actual outer vector capacities.
#[derive(Debug)]
pub(crate) struct CandidateResidence {
    /// Existing execution counters retain no numerical payload.
    trace: crate::execution::Trace,
    /// Combined physical backing bytes, independent of destination elements.
    bytes: usize,
}

impl CandidateResidence {
    /// Register both actual capacities once, without creating another heap allocation.
    pub(crate) fn new<P: Scalar>(
        trace: &crate::execution::Trace,
        outputs: usize,
        statistics: usize,
    ) -> Result<Self, crate::OperationError> {
        let bytes = outputs
            .checked_mul(size_of::<Payload<P>>())
            .and_then(|ordinary| {
                statistics
                    .checked_mul(size_of::<crate::statistics::MomentPayload<P>>())
                    .and_then(|moments| ordinary.checked_add(moments))
            })
            .ok_or(crate::OperationError::ResourceLimit)?;
        trace.retain_residence(bytes)?;
        Ok(Self {
            trace: trace.clone(),
            bytes,
        })
    }
}

impl Drop for CandidateResidence {
    fn drop(&mut self) {
        self.trace.release_residence(self.bytes);
    }
}

impl<P: Scalar> Candidate<P> {
    /// Binding identity; it does not authorize publication by another binding.
    #[must_use]
    pub const fn execution(&self) -> ExecutionId {
        self.execution
    }
    /// Number of private output candidates, including those with companion regions.
    #[must_use]
    pub const fn output_count(&self) -> usize {
        self.outputs.len()
    }
}

/// Private candidate centers and complete supported companions, with future target version.
#[derive(Debug)]
pub(crate) struct Payload<P: Scalar> {
    /// Reserved logical extent.
    pub(crate) extent: Extent,
    /// Exact reserved output identity and version.
    pub(crate) target: ValueStamp,
    /// Unpublished column-ordered centers.
    pub(crate) centers: crate::private::Buffer<P>,
    /// Unpublished complete declared-bound region, if carried by the input.
    pub(crate) bounds: Option<crate::private::Buffer<P>>,
    /// Separate numerical evaluation radii, already included in total bounds.
    pub(crate) evaluation: Option<crate::private::Buffer<P>>,
    /// Three overlapping product derivation regions, when carried or computed.
    pub(crate) contributions: Option<[Vec<P>; 3]>,
}

impl<P: Scalar> Payload<P> {
    /// Every escaped ordinary channel retains its original measured residence debit.
    pub(crate) fn retain_residence(
        &mut self,
        trace: &crate::execution::Trace,
    ) -> Result<(), crate::OperationError> {
        let companions = self
            .contributions
            .iter()
            .flatten()
            .try_fold(0_usize, |sum, values| {
                values
                    .capacity()
                    .checked_mul(size_of::<P>())
                    .and_then(|bytes| sum.checked_add(bytes))
                    .ok_or(crate::OperationError::ResourceLimit)
            })?;
        // Contributions cannot leave this private payload separately. Their debit shares the
        // center's original authority without enlarging any candidate header.
        self.centers.retain_residence(trace, companions)?;
        for values in self.bounds.iter_mut().chain(self.evaluation.iter_mut()) {
            values.retain_residence(trace, 0)?;
        }
        Ok(())
    }
}

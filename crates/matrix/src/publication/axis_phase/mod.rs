// SPDX-License-Identifier: AGPL-3.0-only
//! Exact axis continuations share one complete original source and companion lease.

mod arithmetic;
mod execution;
mod inputs;
mod plan;
pub use inputs::FrozenAxisPhaseInput;
pub use plan::AxisPhasePlan;

use crate::{DecisionSnapshotError, Heap, HeapError, LiveDecisionError};
use nrz_bindings::readings::algebra::Location;
use nrz_bindings::{AxisPhaseAttempt, AxisPhaseOutcome, Budget};
use nrz_scalar::{ArithmeticContext, Enclosure, Finite, Scalar};
use std::{cell::Cell, rc::Rc};

/// One accounting location for bounded numerical preparation and publication.
const AT: Location = Location::new("matrix axis-phase continuation");

/// A complete private mathematical outcome, source lease and certified constant owner.
#[derive(Debug)]
pub struct PreparedAxisPhase<P: Scalar> {
    /// Original normalized-reactance source and complete copy/radius versions.
    frozen: FrozenAxisPhaseInput<P>,
    /// Checked original question, enclosure, outcome and any reciprocal discharge.
    attempt: AxisPhaseAttempt,
    /// Predeclared exact branches and their common numerical pi region.
    plan: AxisPhasePlan<P>,
    /// Entire captured divisor box, retained for the independent numerical guard.
    operand: Enclosure<P>,
    /// Complete new publication allowance, including the retained exact attempt.
    bytes: u64,
}

/// Shared live authority for every dependent phase or reciprocal read.
#[derive(Debug)]
struct Data<P: Scalar> {
    /// All mathematical, numerical and version evidence is one publication unit.
    prepared: PreparedAxisPhase<P>,
    /// A stale source or radius permanently retires this authority.
    retired: Cell<bool>,
}

/// Published explicit outcome; undefined and undecided have no numeric phase placeholder.
#[derive(Clone, Debug)]
pub struct PublishedAxisPhase<P: Scalar>(Rc<Data<P>>);

impl<P: Scalar + 'static> PublishedAxisPhase<P> {
    /// Borrow the original checked outcome only while its complete dependency closure is current.
    /// # Errors
    /// Refuses stale sources or companions, foreign heaps, changed arithmetic and limits.
    pub fn current<'a>(
        &'a self,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<&'a AxisPhaseAttempt, LiveDecisionError> {
        budget.work(1, AT)?;
        if self.0.retired.get() {
            return Err(LiveDecisionError::Retired);
        }
        self.0.prepared.plan.check(context)?;
        if let Err(cause) = heap.check_axis_phase_input(&self.0.prepared.frozen, budget) {
            if !transient(&cause) {
                self.0.retired.set(true);
            }
            return Err(cause.into());
        }
        Ok(&self.0.prepared.attempt)
    }
    /// Exact selected branch enclosed in radians by the retained certified pi owner.
    /// # Errors
    /// Refuses retired authority, undefined/undecided phase, foreign heaps and limits.
    pub fn phase(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Enclosure<P>, LiveDecisionError> {
        match self.current(heap, context, budget)?.outcome() {
            AxisPhaseOutcome::Negative => Ok(self.0.prepared.plan.positive().negated()),
            AxisPhaseOutcome::Positive => Ok(self.0.prepared.plan.positive()),
            AxisPhaseOutcome::Undefined => Err(LiveDecisionError::UndefinedPhase),
            AxisPhaseOutcome::Undecided => Err(LiveDecisionError::UndecidedPhase),
        }
    }
    /// Enclose the original normalized reciprocal after both semantic and numerical guards.
    ///
    /// The returned coordinate is 1/x; physical admittance restoration is a separate map.
    /// A discharged CAS premise never bypasses the scalar division's whole-box zero guard.
    /// # Errors
    /// Refuses undefined/undecided outcomes, stale versions, a zero-containing numerical
    /// divisor, changed arithmetic and resources without returning a reciprocal prefix.
    pub fn reciprocal(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Enclosure<P>, LiveDecisionError> {
        let attempt = self.current(heap, context, budget)?;
        match attempt.outcome() {
            AxisPhaseOutcome::Undefined => return Err(LiveDecisionError::UndefinedPhase),
            AxisPhaseOutcome::Undecided => return Err(LiveDecisionError::UndecidedPhase),
            AxisPhaseOutcome::Negative | AxisPhaseOutcome::Positive => {}
        }
        if attempt.reciprocal().is_none() {
            return Err(LiveDecisionError::Recipe);
        }
        budget.work(8, AT)?;
        Ok(Enclosure::singleton(Finite::ONE).div(self.0.prepared.operand, context)?)
    }
    /// Historical source versions; inspection alone grants no current execution authority.
    #[must_use]
    pub fn inputs(&self) -> &FrozenAxisPhaseInput<P> {
        &self.0.prepared.frozen
    }
    /// Shared original certified pi region; it is a source-free constant.
    #[must_use]
    pub fn plan(&self) -> &AxisPhasePlan<P> {
        &self.0.prepared.plan
    }
    /// Complete retained publication allowance, excluding shared original case/constant owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.prepared.bytes
    }
}

/// A failed foreign-owner or resource probe does not poison a still-current original authority.
fn transient(cause: &DecisionSnapshotError) -> bool {
    let heap = |cause: &HeapError| {
        matches!(
            cause,
            HeapError::ForeignHandle | HeapError::WorkLimit | HeapError::Capacity
        )
    };
    match cause {
        DecisionSnapshotError::Resources(_) => true,
        DecisionSnapshotError::Heap(cause) => heap(cause),
        DecisionSnapshotError::Phasor(cause) => match cause.as_ref() {
            crate::PhasorError::Exact(_)
            | crate::PhasorError::Live(crate::LiveBindingError::Exact(_)) => true,
            crate::PhasorError::Heap(cause)
            | crate::PhasorError::Live(crate::LiveBindingError::Heap(cause)) => heap(cause),
            _ => false,
        },
        _ => false,
    }
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Source-free half-pi enclosure belongs to the predeclared original axis question.

use super::{AT, LiveDecisionError};
use crate::{HeapError, PiConstant};
use nrz_bindings::{AxisPhaseCase, Budget, readings::algebra::limits::BudgetKind};
use nrz_scalar::{ArithmeticContext, Enclosure, Finite, LiteralLimits, Scalar};
use std::rc::Rc;

/// One original exact axis case and compatible certified numerical pi materialization.
#[derive(Clone, Debug)]
pub struct AxisPhasePlan<P: Scalar>(Rc<Data<P>>);

/// Clones share the original constant region and half-pi enclosure without reevaluation.
#[derive(Debug)]
struct Data<P: Scalar> {
    /// Exact physical question, branches and original reciprocal.
    case: AxisPhaseCase,
    /// One immutable certified pi and reciprocal region.
    pi: PiConstant<P>,
    /// Enclosed positive half-pi; negative phase is exact endpoint negation.
    positive: Enclosure<P>,
    /// Bounded endpoint import policy, charged on every attempted live preparation.
    literals: LiteralLimits,
    /// Fixed new plan owner bytes, excluding the shared original pi and case.
    bytes: u64,
}

impl<P: Scalar + 'static> AxisPhasePlan<P> {
    /// Predeclare both numerical branches from the existing certified pi region.
    /// # Errors
    /// Refuses changed arithmetic, nonfinite operations and cumulative caller resources.
    pub fn new(
        case: AxisPhaseCase,
        pi: &PiConstant<P>,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, LiveDecisionError> {
        if pi.arithmetic_revision() != context.revision() {
            return Err(LiveDecisionError::Recipe);
        }
        let bytes = u64::try_from(size_of::<Data<P>>() + 2 * size_of::<usize>())
            .map_err(|_| LiveDecisionError::Snapshot(HeapError::Capacity.into()))?;
        budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        budget.work(8, AT)?;
        let positive = pi
            .pi()
            .enclosure()
            .div(Enclosure::singleton(Finite::new(P::ONE + P::ONE)?), context)?;
        Ok(Self(Rc::new(Data {
            case,
            pi: pi.clone(),
            positive,
            literals,
            bytes,
        })))
    }
    /// Original immutable physical/mathematical declaration.
    #[must_use]
    pub fn case(&self) -> &AxisPhaseCase {
        &self.0.case
    }
    /// Original certified source-free constant, shared across compatible consumers.
    #[must_use]
    pub fn constant(&self) -> &PiConstant<P> {
        &self.0.pi
    }
    /// Fixed new plan owner allowance.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
    /// Numerical evidence never crosses arithmetic proof revisions.
    pub(super) fn check(&self, context: &ArithmeticContext) -> Result<(), LiveDecisionError> {
        if self.0.pi.arithmetic_revision() == context.revision() {
            Ok(())
        } else {
            Err(LiveDecisionError::Recipe)
        }
    }
    /// Exact original positive branch's certified numerical enclosure.
    pub(super) fn positive(&self) -> Enclosure<P> {
        self.0.positive
    }
    /// The original bounded rational endpoint decoding policy.
    pub(super) fn literals(&self) -> LiteralLimits {
        self.0.literals
    }
}

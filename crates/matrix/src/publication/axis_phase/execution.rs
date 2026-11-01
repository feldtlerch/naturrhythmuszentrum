// SPDX-License-Identifier: AGPL-3.0-only
//! Complete exact, numerical and live evidence precedes the atomic external publication.

use super::arithmetic::endpoint;
use super::{AT, AxisPhasePlan, Data, FrozenAxisPhaseInput, PreparedAxisPhase, PublishedAxisPhase};
use crate::publication::decision::arithmetic::enclosure;
use crate::{Coverage, Heap, HeapError, LiveDecisionError};
use nrz_bindings::{
    AxisPhaseAttempts, Budget,
    readings::algebra::{decision::RationalEnclosure, limits::BudgetKind},
};
use nrz_scalar::{ArithmeticContext, Enclosure, Scalar};
use std::{cell::Cell, rc::Rc};

impl Heap {
    /// Match the complete native box to the original certified question and prepare its outcome.
    ///
    /// Numerical-error coverage must enclose the closed exact target. The entire total
    /// box must fit the checked rational enclosure; the center alone never selects a sign.
    /// # Errors
    /// Refuses changed cases, incomplete coverage, mismatched original values, inward or
    /// excluding certificate boxes, stale versions and cumulative resources atomically.
    pub fn prepare_axis_phase<P: Scalar + 'static>(
        &self,
        frozen: FrozenAxisPhaseInput<P>,
        attempts: &mut AxisPhaseAttempts,
        certificate: &RationalEnclosure,
        plan: &AxisPhasePlan<P>,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<PreparedAxisPhase<P>, LiveDecisionError> {
        plan.check(context)?;
        let same_plan = plan.case().same_question(frozen.case());
        let same_attempts = attempts.case().same_question(frozen.case());
        if !same_plan || !same_attempts {
            return Err(LiveDecisionError::Recipe);
        }
        self.check_axis_phase_input(&frozen, budget)?;
        let operand = numerical(&frozen, certificate, plan, context, budget)?;
        let attempt =
            budget.with_retained_payload(frozen.retained_payload_bytes(), AT, |budget| {
                attempts
                    .attempt(certificate, budget)
                    .map_err(|cause| LiveDecisionError::Admission(Box::new(cause)))
            })?;
        let bytes = frozen
            .retained_payload_bytes()
            .checked_add(attempt.retained_payload_bytes())
            .and_then(|bytes| {
                bytes
                    .checked_add(u64::try_from(size_of::<Data<P>>() + 2 * size_of::<usize>()).ok()?)
            })
            .ok_or_else(|| LiveDecisionError::Snapshot(HeapError::Capacity.into()))?;
        budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        Ok(PreparedAxisPhase {
            frozen,
            attempt,
            plan: plan.clone(),
            operand,
            bytes,
        })
    }
    /// Publish the complete outcome and all dependent evidence after the final lease check.
    ///
    /// This externally owned attachment allocates no floating placeholder for undefined
    /// or undecided phase. Existing impedance and independent spectrum records survive.
    /// # Errors
    /// Refuses changed sources or companions and exhausted publication resources without
    /// returning a partial live authority; previously consumed Work remains spent.
    pub fn publish_axis_phase<P: Scalar + 'static>(
        &self,
        prepared: PreparedAxisPhase<P>,
        budget: &mut Budget,
    ) -> Result<PublishedAxisPhase<P>, LiveDecisionError> {
        budget.check(BudgetKind::StagedPayloadBytes, prepared.bytes, AT)?;
        budget.check(BudgetKind::OutputBytes, prepared.bytes, AT)?;
        budget.work(32, AT)?;
        self.check_axis_phase_input(&prepared.frozen, budget)?;
        Ok(PublishedAxisPhase(Rc::new(Data {
            prepared,
            retired: Cell::new(false),
        })))
    }
}

/// Both nominal correspondence and whole-family containment are necessary independent checks.
fn numerical<P: Scalar + 'static>(
    frozen: &FrozenAxisPhaseInput<P>,
    certificate: &RationalEnclosure,
    plan: &AxisPhasePlan<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<Enclosure<P>, LiveDecisionError> {
    let (coordinates, coverage) = frozen.coordinates();
    if coverage != Coverage::DeclaredBounds {
        return Err(LiveDecisionError::Recipe);
    }
    let exact = frozen
        .case()
        .source()
        .exact()
        .and_then(nrz_bindings::ExactConstant::as_rational)
        .ok_or(LiveDecisionError::Recipe)?;
    let original = endpoint::<P>(exact, context, plan.literals(), budget, AT)?;
    let lower = endpoint::<P>(&certificate.lower, context, plan.literals(), budget, AT)?;
    let upper = endpoint::<P>(&certificate.upper, context, plan.literals(), budget, AT)?;
    budget.work(80, AT)?;
    for index in [0, 3] {
        let [center, _, error] = coordinates[index];
        if !enclosure([center, error, error], context)?.contains(nrz_scalar::Finite::ZERO) {
            return Err(LiveDecisionError::Enclosure);
        }
    }
    let mut boxes = [original; 2];
    for (slot, index) in [1, 2].into_iter().enumerate() {
        let [center, total, error] = coordinates[index];
        let nominal = enclosure([center, error, error], context)?;
        let complete = enclosure([center, total, error], context)?;
        let (nominal, complete) = if index == 2 {
            (nominal.negated(), complete.negated())
        } else {
            (nominal, complete)
        };
        let original_covered =
            nominal.lower() <= original.lower() && nominal.upper() >= original.upper();
        let complete_covered =
            complete.lower() >= lower.upper() && complete.upper() <= upper.lower();
        if !original_covered || !complete_covered {
            return Err(LiveDecisionError::Enclosure);
        }
        boxes[slot] = complete;
    }
    let lower = if boxes[0].lower() < boxes[1].lower() {
        boxes[0].lower()
    } else {
        boxes[1].lower()
    };
    let upper = if boxes[0].upper() > boxes[1].upper() {
        boxes[0].upper()
    } else {
        boxes[1].upper()
    };
    Ok(Enclosure::new(lower.get(), upper.get())?)
}

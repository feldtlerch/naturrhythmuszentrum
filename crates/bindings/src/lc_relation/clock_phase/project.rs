// SPDX-License-Identifier: AGPL-3.0-only
//! The offset remains correlated with physical initial phase after effective phase elimination.

mod restricted;

use super::{
    AT, Bounds, Cover, LcClockPhaseCover, LcInterval, LcRelation, LcRelationError, LcSharedClock,
    LcSharedClockPhase, Rational, admit, storage, witness,
};
use crate::{Budget, LcInput, LcQuantity as Q};
use nrz_cas::pi_bounds::PiBoundCertificate;
use std::rc::Rc;

/// Eliminating `beta=alpha-offset` is exact because every beta retains its original offset fibre.
pub(super) fn cover(
    phase: &LcSharedClockPhase,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcClockPhaseCover, LcRelationError> {
    if phase.0.narrow {
        return restricted::cover(phase, cycles, pi, budget);
    }
    let before = budget.usage();
    budget.work(1, AT)?;
    let relation = effective(phase, budget)?;
    let offsets = phase.clock().offset();
    let offset = LcInterval::new(
        Some(offsets[0].clone().into()),
        Some(offsets[1].clone().into()),
        budget,
    )?;
    let clock = LcSharedClock::new(
        relation,
        phase.clock().nominal_times().to_vec(),
        &offset,
        budget,
    )?;
    let effective = clock.pairs_cover(cycles, pi, budget)?;
    let point = effective.feasibility_witness();
    let beta = point
        .original()
        .values()
        .initial_phase
        .as_ref()
        .and_then(crate::ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    let offsets = at_beta(phase, beta, budget)?;
    let witness = witness::from_point(phase, point, &offsets[0], budget)?;
    let bytes = phase
        .retained_payload_bytes()
        .saturating_add(phase.clock().retained_payload_bytes())
        .saturating_add(phase.relation().retained_payload_bytes())
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.offset()))
        .saturating_add(storage::rational(witness.original().offset()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcClockPhaseCover(Rc::new(Cover {
        phase: phase.clone(),
        effective,
        measured: None,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}

/// Effective sample phases and initial-phase support keep all original time and scale constraints.
fn effective(
    phase: &LcSharedClockPhase,
    budget: &mut Budget,
) -> Result<LcRelation, LcRelationError> {
    storage::retained(
        phase
            .relation()
            .retained_payload_bytes()
            .saturating_mul(2)
            .saturating_add(phase.retained_payload_bytes()),
        budget,
    )?;
    let mut inputs = Vec::new();
    for input in phase.relation().inputs() {
        budget.work(1, AT)?;
        if !matches!(input.quantity(), Q::Phase(_) | Q::InitialPhase) {
            inputs.push(input.clone());
        }
    }
    for (index, interval) in phase.samples().iter().enumerate() {
        let quantity = Q::Phase(index);
        inputs.push(LcInput::new(
            quantity,
            &quantity.descriptor()?,
            interval.clone(),
            budget,
        )?);
    }
    if let Some([lower, upper]) = &phase.0.initial {
        let range = [
            lower.sub(&phase.offset()[1], budget)?,
            upper.sub(&phase.offset()[0], budget)?,
        ];
        let [lower, upper] = range;
        let interval = LcInterval::new(Some(lower.into()), Some(upper.into()), budget)?;
        let quantity = Q::InitialPhase;
        inputs.push(LcInput::new(
            quantity,
            &quantity.descriptor()?,
            interval,
            budget,
        )?);
    }
    LcRelation::new(
        phase.relation().epoch().into(),
        phase.relation().observations(),
        inputs,
        budget,
    )
}

/// A beta inside `A-S` has a nonempty fibre `S intersect (A-beta)`.
fn at_beta(
    phase: &LcSharedClockPhase,
    beta: &Rational,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    budget.work(1, AT)?;
    let mut offset = phase.offset().clone();
    if let Some([lower, upper]) = &phase.0.initial {
        let range = [lower.sub(beta, budget)?, upper.sub(beta, budget)?];
        admit::intersect(&mut offset, &range, budget)?;
    }
    Ok(offset)
}

/// A refinement reuses checked pi evidence only in the supported physical-phase projection.
pub(super) fn cover_checked(
    phase: &LcSharedClockPhase,
    cycles: Vec<Rational>,
    pi: nrz_cas::pi_bounds::CheckedPiBounds,
    budget: &mut Budget,
) -> Result<LcClockPhaseCover, LcRelationError> {
    budget.work(1, AT)?;
    if !phase.0.narrow {
        return Err(LcRelationError::Projection);
    }
    restricted::cover_checked(phase, cycles, pi, budget)
}

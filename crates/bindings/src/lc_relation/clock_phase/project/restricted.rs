// SPDX-License-Identifier: AGPL-3.0-only
//! Physical and measured intercept strips retain one common phase offset at every recovered rate.

mod source;

use super::super::{
    AT, Bounds, Cover, LcClockPhaseCover, LcRelationError, LcSharedClockPhase, Rational, admit,
    storage, witness,
};
use crate::{Budget, LcPairsCover};
use nrz_cas::pi_bounds::PiBoundCertificate;
use nrz_cas::{limits::BudgetUsage, pi_bounds::CheckedPiBounds};
use std::rc::Rc;

/// Measured-strip nonemptiness and a shared physical intercept jointly prove the exact domain.
pub(super) fn cover(
    phase: &LcSharedClockPhase,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcClockPhaseCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(
        phase
            .relation()
            .retained_payload_bytes()
            .saturating_mul(3)
            .saturating_add(phase.retained_payload_bytes())
            .saturating_add(32_768),
        budget,
    )?;
    let measured = source::measured(phase, budget)?.pairs_cover(cycles.clone(), pi, budget)?;
    finish(phase, measured, cycles, &before, budget)
}

/// Prepared circle evidence is reused, with the same complete independent measured certificate.
pub(super) fn cover_checked(
    phase: &LcSharedClockPhase,
    cycles: Vec<Rational>,
    pi: CheckedPiBounds,
    budget: &mut Budget,
) -> Result<LcClockPhaseCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(
        phase
            .relation()
            .retained_payload_bytes()
            .saturating_mul(3)
            .saturating_add(phase.retained_payload_bytes())
            .saturating_add(32_768),
        budget,
    )?;
    let measured =
        source::measured(phase, budget)?.pairs_cover_checked(cycles.clone(), pi, budget)?;
    finish(phase, measured, cycles, &before, budget)
}

/// Original reconstruction follows identical exact elimination for raw and prepared pi evidence.
fn finish(
    phase: &LcSharedClockPhase,
    measured: LcPairsCover,
    cycles: Vec<Rational>,
    before: &BudgetUsage,
    budget: &mut Budget,
) -> Result<LcClockPhaseCover, LcRelationError> {
    let effective = source::physical(phase, measured.rate(), budget)?.pairs_cover_checked(
        cycles,
        measured.pi_bounds().clone(),
        budget,
    )?;
    let point = effective.feasibility_witness();
    let values = point.original().values();
    let rate = values
        .rate
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let initial = values
        .initial_phase
        .as_ref()
        .and_then(crate::ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    let intercept = initial.add(&rate.mul(point.offset(), budget)?, budget)?;
    let samples = measured.phase_at_rate(rate, budget)?;
    let offsets = at_intercept(phase, &intercept, &samples, budget)?;
    let witness = witness::check(
        phase,
        point.original().values().clone(),
        point.offset().clone(),
        offsets[0].clone(),
        budget,
    )?;
    let bytes = phase
        .retained_payload_bytes()
        .saturating_add(phase.clock().retained_payload_bytes())
        .saturating_add(phase.relation().retained_payload_bytes())
        .saturating_add(measured.retained_payload_bytes())
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.offset()))
        .saturating_add(storage::rational(witness.original().offset()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcClockPhaseCover(Rc::new(Cover {
        phase: phase.clone(),
        measured: Some(measured),
        effective,
        witness,
        usage: [*before, budget.usage()],
        bytes,
    })))
}

/// A physical intercept in the expanded measured strips has one common additive-offset fibre.
fn at_intercept(
    phase: &LcSharedClockPhase,
    intercept: &Rational,
    measured: &Bounds,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    budget.work(1, AT)?;
    let mut offset = [
        intercept.sub(&measured[1], budget)?,
        intercept.sub(&measured[0], budget)?,
    ];
    admit::intersect(&mut offset, phase.offset(), budget)?;
    Ok(offset)
}

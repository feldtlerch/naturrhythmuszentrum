// SPDX-License-Identifier: AGPL-3.0-only
//! One original clock/phase question retains all three shared errors through positive scale restoration.

use super::{
    AT, Cover, LcRelationError, LcSharedMeasurementCover, LcSharedMeasurements, Rational, admit,
    storage, witness,
};
use crate::LcClockPhaseCover;
use crate::{Budget, LcInterval, LcQuantity as Q, LcRelation, LcSharedClock, LcSharedClockPhase};
use nrz_cas::{
    limits::BudgetUsage,
    pi_bounds::{CheckedPiBounds, PiBoundCertificate},
};
use std::rc::Rc;

/// All omitted physical inputs are exactly represented in the retained amplitude owner.
pub(super) fn cover(
    amplitude: &LcSharedMeasurements,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcSharedMeasurementCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(
        amplitude
            .relation()
            .retained_payload_bytes()
            .saturating_mul(2),
        budget,
    )?;
    let phase = effective_question(amplitude, budget)?;
    let effective = phase.pairs_cover(cycles, pi, budget)?;
    finish(amplitude, effective, &before, budget)
}

/// Reuse checked circle evidence for a complete original calibrated refinement.
pub(super) fn cover_checked(
    amplitude: &LcSharedMeasurements,
    cycles: Vec<Rational>,
    pi: CheckedPiBounds,
    budget: &mut Budget,
) -> Result<LcSharedMeasurementCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(
        amplitude
            .relation()
            .retained_payload_bytes()
            .saturating_mul(2),
        budget,
    )?;
    let phase = effective_question(amplitude, budget)?;
    let effective = phase.pairs_cover_checked(cycles, pi, budget)?;
    finish(amplitude, effective, &before, budget)
}

/// Every calibration and original physical equation is checked on the restored feasibility tuple.
fn finish(
    amplitude: &LcSharedMeasurements,
    effective: LcClockPhaseCover,
    before: &BudgetUsage,
    budget: &mut Budget,
) -> Result<LcSharedMeasurementCover, LcRelationError> {
    let energy = amplitude
        .energy()
        .map_or_else(Rational::one, |range| range[0].clone());
    let physical = &amplitude.physical_amplitude()[0];
    let gain = admit::gain_at_amplitude(amplitude, physical, budget)?;
    let measured = physical.div(&gain[0], budget)?;
    let witness = witness::from_point(
        amplitude,
        effective.feasibility_witness(),
        &measured,
        &gain[0],
        &energy,
        budget,
    )?;
    let bytes = amplitude
        .retained_payload_bytes()
        .saturating_add(amplitude.phase().retained_payload_bytes())
        .saturating_add(amplitude.phase().clock().retained_payload_bytes())
        .saturating_add(amplitude.relation().retained_payload_bytes())
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(
            witness
                .original()
                .original()
                .original()
                .retained_payload_bytes(),
        )
        .saturating_add(storage::rational(witness.original().offset()))
        .saturating_add(storage::rational(witness.original().original().offset()))
        .saturating_add(storage::rational(witness.gain()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcSharedMeasurementCover(Rc::new(Cover {
        amplitude: amplitude.clone(),
        effective,
        witness,
        usage: [*before, budget.usage()],
        bytes,
    })))
}

/// Removing only retained positive scale restrictions preserves the original clock and phase question.
fn effective_question(
    amplitude: &LcSharedMeasurements,
    budget: &mut Budget,
) -> Result<LcSharedClockPhase, LcRelationError> {
    budget.work(1, AT)?;
    let mut inputs = Vec::new();
    for input in amplitude.relation().inputs() {
        budget.work(1, AT)?;
        let measured_channel = input.quantity() == amplitude.channel();
        let energy = input.quantity() == Q::Energy;
        if !measured_channel && !energy {
            inputs.push(input.clone());
        }
    }
    let relation = LcRelation::new(
        amplitude.relation().epoch().into(),
        amplitude.relation().observations(),
        inputs,
        budget,
    )?;
    let original_clock = amplitude.phase().clock();
    let clock_offset = LcInterval::new(
        Some(original_clock.offset()[0].clone().into()),
        Some(original_clock.offset()[1].clone().into()),
        budget,
    )?;
    let clock = LcSharedClock::new(
        relation,
        original_clock.nominal_times().to_vec(),
        &clock_offset,
        budget,
    )?;
    let phase_offset = LcInterval::new(
        Some(amplitude.phase().offset()[0].clone().into()),
        Some(amplitude.phase().offset()[1].clone().into()),
        budget,
    )?;
    LcSharedClockPhase::new(
        clock,
        amplitude.phase().samples().to_vec(),
        &phase_offset,
        budget,
    )
}

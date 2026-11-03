// SPDX-License-Identifier: AGPL-3.0-only
//! Interval intersections retain original actual times, independent errors and one shared offset jointly.

mod source;

use super::super::{
    AT, Cover, LcRelationError, LcSharedTiming, LcSharedTimingCover, LcSharedTimingWitness,
    Rational, storage,
};
use crate::lc_relation::calibration;
use crate::{Budget, LcClockPhaseParameters, LcPairParameters, LcTimingCover};
use nrz_cas::{
    limits::BudgetUsage,
    pi_bounds::{CheckedPiBounds, PiBoundCertificate},
};
use std::rc::Rc;

/// Nonempty effective intercept strips and exact physical intersections give one shared-offset fibre.
pub(super) fn cover(
    timing: &LcSharedTiming,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcSharedTimingCover, LcRelationError> {
    let before = budget.usage();
    preflight(timing, budget)?;
    let measured = source::measured(timing, budget)?.pairs_cover(cycles.clone(), pi, budget)?;
    finish(timing, cycles, measured, &before, budget)
}

/// Reuse previous circle evidence in both complete strip owners of the refined question.
pub(super) fn checked(
    timing: &LcSharedTiming,
    cycles: Vec<Rational>,
    pi: CheckedPiBounds,
    budget: &mut Budget,
) -> Result<LcSharedTimingCover, LcRelationError> {
    let before = budget.usage();
    preflight(timing, budget)?;
    let measured =
        source::measured(timing, budget)?.pairs_cover_checked(cycles.clone(), pi, budget)?;
    finish(timing, cycles, measured, &before, budget)
}

/// Reserve storage for both complete original strip descriptions before solving.
fn preflight(timing: &LcSharedTiming, budget: &mut Budget) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    storage::retained(
        timing
            .relation()
            .retained_payload_bytes()
            .saturating_mul(3)
            .saturating_add(timing.retained_payload_bytes())
            .saturating_add(32_768),
        budget,
    )?;
    Ok(())
}

/// Conditional restoration retains one shared offset and all narrowed original time inputs.
fn finish(
    timing: &LcSharedTiming,
    cycles: Vec<Rational>,
    measured: LcTimingCover,
    before: &BudgetUsage,
    budget: &mut Budget,
) -> Result<LcSharedTimingCover, LcRelationError> {
    let effective = source::physical(timing, measured.rate(), budget)?.pairs_cover_checked(
        cycles,
        measured.pi_bounds().clone(),
        budget,
    )?;
    let rate = effective.rate()[0].clone();
    let alpha = effective.phase_at_rate(&rate, budget)?[0].clone();
    let beta = measured.phase_at_rate(&rate, budget)?;
    let mut offset = timing.offset_support().clone();
    let image = [
        beta[0].sub(&alpha, budget)?.div(&rate, budget)?,
        beta[1].sub(&alpha, budget)?.div(&rate, budget)?,
    ];
    calibration::intersect(&mut offset, &image, budget)?;
    let parameters = LcClockPhaseParameters {
        rate,
        initial_phase: alpha,
        gauge: Rational::zero(),
        offset: offset[0].clone(),
        ratio: Rational::one(),
        voltage: Rational::one(),
    };
    let witness = restore(timing, &effective, &parameters, budget)?;
    let bytes = timing
        .retained_payload_bytes()
        .saturating_add(timing.relation().retained_payload_bytes())
        .saturating_add(measured.retained_payload_bytes())
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.offset()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcSharedTimingCover(Rc::new(Cover {
        timing: timing.clone(),
        effective,
        measured: Some(measured),
        witness,
        usage: [*before, budget.usage()],
        bytes,
    })))
}

/// Both actual and measured strip owners validate the selected shared offset before reconstruction.
pub(in crate::lc_relation::timing::shared) fn witness(
    cover: &LcSharedTimingCover,
    parameters: &LcClockPhaseParameters,
    budget: &mut Budget,
) -> Result<LcSharedTimingWitness, LcRelationError> {
    let offsets = cover.offset_at_phase(&parameters.rate, &parameters.initial_phase, budget)?;
    crate::lc_relation::phase::admit::contains(&offsets, &parameters.offset, budget)?;
    restore(cover.timing(), &cover.0.effective, parameters, budget)
}

/// Pairwise intersections of three real intervals give one actual time for every original sample.
fn restore(
    timing: &LcSharedTiming,
    physical: &LcTimingCover,
    parameters: &LcClockPhaseParameters,
    budget: &mut Budget,
) -> Result<LcSharedTimingWitness, LcRelationError> {
    budget.work(1, AT)?;
    storage::retained(
        storage::size(timing.errors().len())?.saturating_mul(2_048),
        budget,
    )?;
    let mut times = Vec::with_capacity(timing.errors().len());
    for (nominal, error) in timing.nominal_times().iter().zip(timing.errors()) {
        budget.work(1, AT)?;
        times.push([
            nominal
                .add(&error[0], budget)?
                .add(&parameters.offset, budget)?,
            nominal
                .add(&error[1], budget)?
                .add(&parameters.offset, budget)?,
        ]);
    }
    let selected = LcPairParameters {
        rate: parameters.rate.clone(),
        initial_phase: parameters.initial_phase.clone(),
        gauge: parameters.gauge.clone(),
        ratio: parameters.ratio.clone(),
        voltage: parameters.voltage.clone(),
    };
    let point = physical.witness_in_time_boxes(&selected, &times, budget)?;
    timing.check_witness(point.values().clone(), parameters.offset.clone(), budget)
}

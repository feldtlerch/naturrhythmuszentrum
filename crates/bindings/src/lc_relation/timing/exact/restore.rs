// SPDX-License-Identifier: AGPL-3.0-only
//! Positive division restores an original timestamp before any feasibility claim.

use super::{
    AT, Bounds, LcPairParameters, LcPairsCover, LcRelationError, LcSampleTimes, LcWitness, Rational,
};
use crate::{Budget, ExactConstant};
use std::cmp::Ordering;

/// Every retained phase/rate point has an original timestamp in each independent box.
pub(super) fn witness(
    timing: &LcSampleTimes,
    effective: &LcPairsCover,
    phases: &[Bounds],
    cycles: &[Rational],
    shifts: &[ExactConstant],
    parameters: &LcPairParameters,
    budget: &mut Budget,
) -> Result<LcWitness, LcRelationError> {
    budget.work(1, AT)?;
    let point = effective.witness(parameters, budget)?;
    from_point(timing, &point, phases, cycles, shifts, parameters, budget)
}

/// Reuse a checked auxiliary physical tuple; reconstructed original equations are checked afresh.
pub(super) fn from_point(
    timing: &LcSampleTimes,
    point: &LcWitness,
    phases: &[Bounds],
    cycles: &[Rational],
    shifts: &[ExactConstant],
    parameters: &LcPairParameters,
    budget: &mut Budget,
) -> Result<LcWitness, LcRelationError> {
    budget.work(1, AT)?;
    let mut values = point.values().clone();
    values
        .observations
        .truncate(timing.relation().observations());
    for (index, observation) in values.observations.iter_mut().enumerate() {
        budget.work(1, AT)?;
        let time = timestamp(
            &timing.time_boxes()[index],
            &phases[index],
            parameters,
            budget,
        )?;
        let lifted = parameters
            .initial_phase
            .add(&parameters.rate.mul(&time, budget)?, budget)?;
        observation.time = time.into();
        observation.phase = Some(ExactConstant::from(lifted).sub(&shifts[index], budget)?);
        observation.cycle = Some(cycles[index].add(&parameters.gauge, budget)?);
    }
    timing.relation().check_witness(values, budget)
}

/// Select an exact member of `[t_min,t_max] intersect [(p_min-alpha)/a,(p_max-alpha)/a]`.
fn timestamp(
    time: &Bounds,
    phase: &Bounds,
    parameters: &LcPairParameters,
    budget: &mut Budget,
) -> Result<Rational, LcRelationError> {
    let mut lower = phase[0]
        .sub(&parameters.initial_phase, budget)?
        .div(&parameters.rate, budget)?;
    let mut upper = phase[1]
        .sub(&parameters.initial_phase, budget)?
        .div(&parameters.rate, budget)?;
    if time[0].compare(&lower, budget)? == Ordering::Greater {
        lower.clone_from(&time[0]);
    }
    if time[1].compare(&upper, budget)? == Ordering::Less {
        upper.clone_from(&time[1]);
    }
    if lower.compare(&upper, budget)? == Ordering::Greater {
        return Err(LcRelationError::Containment);
    }
    Ok(lower)
}

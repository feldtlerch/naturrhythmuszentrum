// SPDX-License-Identifier: AGPL-3.0-only
//! Physical timestamps restore the same offset and satisfy their own separate error boxes.

use super::{
    AT, LcClockParameters, LcRelationError, LcSharedTiming, LcSharedTimingWitness, LcTimingCover,
    Rational, storage,
};
use crate::{Budget, ExactConstant, LcPairParameters, LcValues};
use std::cmp::Ordering;

/// Original membership alone is insufficient for the additional joint timing model.
pub(super) fn check(
    timing: &LcSharedTiming,
    values: LcValues,
    offset: Rational,
    budget: &mut Budget,
) -> Result<LcSharedTimingWitness, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    offset.check(budget, AT)?;
    if values.observations.len() != timing.nominal_times().len() {
        return Err(LcRelationError::Observation);
    }
    let above = offset.compare(&timing.offset()[0], budget)? != Ordering::Less;
    let below = offset.compare(&timing.offset()[1], budget)? != Ordering::Greater;
    if !above || !below {
        return Err(LcRelationError::Containment);
    }
    let exact_offset = ExactConstant::from(offset.clone());
    for (index, observation) in values.observations.iter().enumerate() {
        budget.work(1, AT)?;
        let delta = observation
            .time
            .sub(&timing.nominal_times()[index].clone().into(), budget)?
            .sub(&exact_offset, budget)?;
        let error = &timing.errors()[index];
        let above = crate::lc_relation::ordered(&error[0].clone().into(), &delta, budget)?;
        let below = crate::lc_relation::ordered(&delta, &error[1].clone().into(), budget)?;
        if !above || !below {
            return Err(LcRelationError::Containment);
        }
    }
    let original = timing.relation().check_witness(values, budget)?;
    let bytes = timing
        .retained_payload_bytes()
        .saturating_add(timing.relation().retained_payload_bytes())
        .saturating_add(original.retained_payload_bytes())
        .saturating_add(storage::rational(&offset))
        .saturating_add(512);
    storage::retained(bytes, budget)?;
    Ok(LcSharedTimingWitness {
        timing: timing.clone(),
        original,
        offset,
        usage: [before, budget.usage()],
    })
}

/// The inverse change of coordinates restores every original physical equation before publication.
pub(super) fn parameters(
    timing: &LcSharedTiming,
    effective: &LcTimingCover,
    parameters: &LcClockParameters,
    budget: &mut Budget,
) -> Result<LcSharedTimingWitness, LcRelationError> {
    budget.work(1, AT)?;
    let beta = parameters.rate.mul(&parameters.offset, budget)?;
    let point = effective.witness(
        &LcPairParameters {
            rate: parameters.rate.clone(),
            initial_phase: beta,
            ratio: parameters.ratio.clone(),
            voltage: parameters.voltage.clone(),
            gauge: Rational::zero(),
        },
        budget,
    )?;
    from_point(timing, &point, &parameters.offset, budget)
}

/// Reuse a previously checked effective tuple while rechecking all restored original equations.
pub(super) fn from_point(
    timing: &LcSharedTiming,
    point: &crate::LcWitness,
    offset: &Rational,
    budget: &mut Budget,
) -> Result<LcSharedTimingWitness, LcRelationError> {
    budget.work(1, AT)?;
    let mut values = point.values().clone();
    values.initial_phase = Some(ExactConstant::zero());
    values
        .observations
        .truncate(timing.relation().observations());
    let exact_offset = ExactConstant::from(offset.clone());
    for observation in &mut values.observations {
        budget.work(1, AT)?;
        observation.time = observation.time.add(&exact_offset, budget)?;
    }
    check(timing, values, offset.clone(), budget)
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Every original timestamp uses the identical checked offset in physical witnesses.

use super::{
    AT, LcClockParameters, LcClockPhaseParameters, LcClockWitness, LcPairsCover, LcRelationError,
    LcSharedClock, Rational, storage,
};
use crate::{Budget, ExactConstant, LcPairParameters, LcValues};
use std::cmp::Ordering;

/// Independent timestamp boxes alone cannot establish this additional joint equation.
pub(super) fn check(
    clock: &LcSharedClock,
    values: LcValues,
    offset: Rational,
    budget: &mut Budget,
) -> Result<LcClockWitness, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    if values.observations.len() != clock.nominal_times().len() {
        return Err(LcRelationError::Observation);
    }
    let above = offset.compare(&clock.offset_support()[0], budget)? != Ordering::Less;
    let below = offset.compare(&clock.offset_support()[1], budget)? != Ordering::Greater;
    if !above || !below {
        return Err(LcRelationError::Containment);
    }
    for (observation, nominal) in values.observations.iter().zip(clock.nominal_times()) {
        budget.work(1, AT)?;
        let expected = ExactConstant::from(nominal.add(&offset, budget)?);
        if !observation.time.structurally_equal(&expected, budget)? {
            return Err(LcRelationError::Containment);
        }
    }
    let original = clock.relation().check_witness(values, budget)?;
    let bytes = original
        .retained_payload_bytes()
        .saturating_add(storage::rational(&offset))
        .saturating_add(clock.retained_payload_bytes())
        .saturating_add(clock.relation().retained_payload_bytes())
        .saturating_add(1_024);
    storage::retained(bytes, budget)?;
    Ok(LcClockWitness {
        clock: clock.clone(),
        original,
        offset,
        usage: [before, budget.usage()],
    })
}

/// The invertible map `beta=a*offset` preserves every real observation and all physical scales.
pub(super) fn parameters(
    clock: &LcSharedClock,
    effective: &LcPairsCover,
    parameters: &LcClockParameters,
    budget: &mut Budget,
) -> Result<LcClockWitness, LcRelationError> {
    let parameters = LcClockPhaseParameters {
        rate: parameters.rate.clone(),
        initial_phase: Rational::zero(),
        gauge: Rational::zero(),
        offset: parameters.offset.clone(),
        ratio: parameters.ratio.clone(),
        voltage: parameters.voltage.clone(),
    };
    with_phase(clock, effective, &parameters, budget)
}

/// Restore the original physical alpha and every timestamp from one joint alpha/offset selection.
pub(super) fn with_phase(
    clock: &LcSharedClock,
    effective: &LcPairsCover,
    parameters: &LcClockPhaseParameters,
    budget: &mut Budget,
) -> Result<LcClockWitness, LcRelationError> {
    budget.work(1, AT)?;
    let shift = parameters.rate.mul(&parameters.offset, budget)?;
    let beta = parameters.initial_phase.add(&shift, budget)?;
    let witness = effective.witness(
        &LcPairParameters {
            rate: parameters.rate.clone(),
            initial_phase: beta,
            ratio: parameters.ratio.clone(),
            voltage: parameters.voltage.clone(),
            gauge: parameters.gauge.clone(),
        },
        budget,
    )?;
    let mut values = witness.values().clone();
    values.initial_phase = Some(
        values
            .initial_phase
            .as_ref()
            .ok_or(LcRelationError::Projection)?
            .sub(&shift.into(), budget)?,
    );
    values
        .observations
        .truncate(clock.relation().observations());
    for (observation, nominal) in values.observations.iter_mut().zip(clock.nominal_times()) {
        budget.work(1, AT)?;
        observation.time = nominal.add(&parameters.offset, budget)?.into();
    }
    check(clock, values, parameters.offset.clone(), budget)
}

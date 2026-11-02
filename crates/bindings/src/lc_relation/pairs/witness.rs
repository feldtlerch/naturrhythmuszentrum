// SPDX-License-Identifier: AGPL-3.0-only
//! The full original LC tuple is reconstructed and independently checked at each parameter.

use super::{
    Bounds, LcPairParameters, LcRelation, LcRelationError, LcWitness, Rational, admit::Inputs,
    project, support,
};
use crate::{Budget, ExactConstant, LcObservationValue, LcValues};

/// Every positive ratio and amplitude extends the retained correlated phase/rate manifold.
pub(super) fn check(
    relation: &LcRelation,
    inputs: &Inputs,
    cycles: &[Rational],
    rate: &Bounds,
    parameters: &LcPairParameters,
    budget: &mut Budget,
) -> Result<LcWitness, LcRelationError> {
    if !support::contains(rate, &parameters.rate, budget)? {
        return Err(LcRelationError::Containment);
    }
    let phase = project::phase(&inputs.strips, &parameters.rate, budget)?;
    if !support::contains(&phase, &parameters.initial_phase, budget)? {
        return Err(LcRelationError::Containment);
    }
    support::integer(&parameters.gauge, budget)?;
    if inputs.fixed_phase && !parameters.gauge.is_zero() {
        return Err(LcRelationError::Cycle);
    }
    for value in [&parameters.ratio, &parameters.voltage] {
        if value.is_negative() || value.is_zero() {
            return Err(LcRelationError::Positivity);
        }
    }
    let turn = support::turn(budget)?;
    let initial = ExactConstant::from(parameters.initial_phase.clone())
        .add(&turn.mul(&parameters.gauge.clone().into(), budget)?, budget)?;
    let mut observations = Vec::with_capacity(relation.observations());
    for (strip, cycle) in inputs.strips.iter().zip(cycles) {
        let lifted = parameters
            .initial_phase
            .add(&parameters.rate.mul(&strip.time, budget)?, budget)?;
        let phase =
            ExactConstant::from(lifted).sub(&turn.mul(&cycle.clone().into(), budget)?, budget)?;
        observations.push(LcObservationValue {
            time: strip.time.clone().into(),
            phase: Some(phase),
            cycle: Some(cycle.add(&parameters.gauge, budget)?),
        });
    }
    let physical = inputs.scale.values(parameters, budget)?;
    relation.check_witness(
        LcValues {
            inductance: physical.inductance.into(),
            capacitance: physical.capacitance.into(),
            rate: parameters.rate.clone().into(),
            initial_phase: Some(initial),
            voltage: physical.voltage,
            current: physical.current,
            energy: physical.energy,
            observations,
        },
        budget,
    )
}

// SPDX-License-Identifier: AGPL-3.0-only
//! One retained phase offset is restored into every physical phase before original checking.

use super::{
    AT, LcClockCover, LcClockPhaseWitness, LcRelationError, LcSharedClockPhase,
    LcSharedClockPhaseParameters, Rational, admit, storage,
};
use crate::{Budget, ExactConstant, LcValues};

/// Check every per-sample phase interval using the same declared systematic offset.
pub(super) fn check(
    phase: &LcSharedClockPhase,
    values: LcValues,
    clock_offset: Rational,
    offset: Rational,
    budget: &mut Budget,
) -> Result<LcClockPhaseWitness, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    if values.observations.len() != phase.samples().len() {
        return Err(LcRelationError::Observation);
    }
    admit::contains(phase.offset(), &offset, budget)?;
    let exact_offset = ExactConstant::from(offset.clone());
    for (observation, interval) in values.observations.iter().zip(phase.samples()) {
        budget.work(1, AT)?;
        let observed = observation
            .phase
            .as_ref()
            .ok_or(LcRelationError::QuiescentPhase)?;
        let measured = observed.sub(&exact_offset, budget)?;
        if !interval.contains(&measured, budget)? {
            return Err(LcRelationError::Containment);
        }
    }
    let original = phase.clock().check_witness(values, clock_offset, budget)?;
    let bytes = phase
        .retained_payload_bytes()
        .saturating_add(phase.clock().retained_payload_bytes())
        .saturating_add(phase.relation().retained_payload_bytes())
        .saturating_add(original.original().retained_payload_bytes())
        .saturating_add(storage::rational(&offset))
        .saturating_add(storage::rational(original.offset()))
        .saturating_add(512);
    storage::retained(bytes, budget)?;
    Ok(LcClockPhaseWitness {
        phase: phase.clone(),
        original,
        offset,
        usage: [before, budget.usage()],
    })
}

/// Offset correlation is checked in effective coordinates, then in the original physical relation.
pub(super) fn parameters(
    phase: &LcSharedClockPhase,
    effective: &LcClockCover,
    parameters: &LcSharedClockPhaseParameters,
    budget: &mut Budget,
) -> Result<LcClockPhaseWitness, LcRelationError> {
    budget.work(1, AT)?;
    admit::contains(phase.offset(), &parameters.offset, budget)?;
    let mut effective_parameters = parameters.physical.clone();
    effective_parameters.initial_phase = parameters
        .physical
        .initial_phase
        .sub(&parameters.offset, budget)?;
    let point = effective.witness_with_phase(&effective_parameters, budget)?;
    from_point(phase, &point, &parameters.offset, budget)
}

/// Restoring a shared phase offset preserves the equations; all original inputs are rechecked.
pub(super) fn from_point(
    phase: &LcSharedClockPhase,
    point: &crate::LcClockWitness,
    offset: &Rational,
    budget: &mut Budget,
) -> Result<LcClockPhaseWitness, LcRelationError> {
    budget.work(1, AT)?;
    let mut values = point.original().values().clone();
    let exact_offset = ExactConstant::from(offset.clone());
    let initial = values
        .initial_phase
        .as_ref()
        .ok_or(LcRelationError::QuiescentPhase)?;
    values.initial_phase = Some(initial.add(&exact_offset, budget)?);
    for observation in &mut values.observations {
        budget.work(1, AT)?;
        let measured = observation
            .phase
            .as_ref()
            .ok_or(LcRelationError::QuiescentPhase)?;
        observation.phase = Some(measured.add(&exact_offset, budget)?);
    }
    check(
        phase,
        values,
        point.offset().clone(),
        offset.clone(),
        budget,
    )
}

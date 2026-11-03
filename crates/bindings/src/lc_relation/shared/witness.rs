// SPDX-License-Identifier: AGPL-3.0-only
//! Every reconstructed physical scale and every measured amplitude share one checked gain.

use super::super::amplitude::scale;
use super::{
    AT, LcRelationError, LcSharedMeasurementCover, LcSharedMeasurementParameters,
    LcSharedMeasurementWitness, LcSharedMeasurements, Rational, admit, storage,
};
use crate::{
    Budget, ExactConstant, LcClockPhaseParameters, LcClockPhaseWitness, LcQuantity as Q,
    LcSharedClockPhaseParameters, LcValues,
};

/// Independent repeated supports are checked with one identical positive calibration gain.
pub(super) fn check(
    amplitude: &LcSharedMeasurements,
    values: LcValues,
    clock_offset: Rational,
    phase_offset: Rational,
    gain: Rational,
    budget: &mut Budget,
) -> Result<LcSharedMeasurementWitness, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    admit::contains(amplitude.gain(), &gain, budget)?;
    let actual = if amplitude.channel() == Q::VoltageAmplitude {
        &values.voltage
    } else {
        &values.current
    };
    let measured = actual.div(&ExactConstant::from(gain.clone()), budget)?;
    for sample in amplitude.samples() {
        budget.work(1, AT)?;
        if !sample.contains(&measured, budget)? {
            return Err(LcRelationError::Containment);
        }
    }
    let original = amplitude
        .phase()
        .check_witness(values, clock_offset, phase_offset, budget)?;
    let bytes = amplitude
        .retained_payload_bytes()
        .saturating_add(amplitude.phase().retained_payload_bytes())
        .saturating_add(amplitude.phase().clock().retained_payload_bytes())
        .saturating_add(amplitude.relation().retained_payload_bytes())
        .saturating_add(original.original().original().retained_payload_bytes())
        .saturating_add(storage::rational(&gain))
        .saturating_add(storage::rational(original.offset()))
        .saturating_add(storage::rational(original.original().offset()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcSharedMeasurementWitness {
        amplitude: amplitude.clone(),
        original,
        gain,
        usage: [before, budget.usage()],
    })
}

/// Phase coordinates remain original; only exact positive physical scale coordinates change.
pub(super) fn from_point(
    amplitude: &LcSharedMeasurements,
    point: &LcClockPhaseWitness,
    measured: &Rational,
    gain: &Rational,
    energy: &Rational,
    budget: &mut Budget,
) -> Result<LcSharedMeasurementWitness, LcRelationError> {
    admit::contains(amplitude.measured(), measured, budget)?;
    admit::contains(amplitude.gain(), gain, budget)?;
    admit::positive(energy, budget)?;
    if let Some(range) = amplitude.energy() {
        admit::contains(range, energy, budget)?;
    }
    let mut values = point.original().original().values().clone();
    let rate = values
        .rate
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let actual = measured.mul(gain, budget)?;
    let channel = amplitude.channel();
    values.inductance =
        scale::coordinate(channel, Q::Inductance, rate, &actual, energy, budget)?.into();
    values.capacitance =
        scale::coordinate(channel, Q::Capacitance, rate, &actual, energy, budget)?.into();
    values.voltage =
        scale::coordinate(channel, Q::VoltageAmplitude, rate, &actual, energy, budget)?.into();
    values.current =
        scale::coordinate(channel, Q::CurrentAmplitude, rate, &actual, energy, budget)?.into();
    values.energy = energy.clone().into();
    check(
        amplitude,
        values,
        point.original().offset().clone(),
        point.offset().clone(),
        gain.clone(),
        budget,
    )
}

/// The full joint domain is parameterized explicitly, so no marginal product supplies a witness.
pub(super) fn parameters(
    cover: &LcSharedMeasurementCover,
    parameters: &LcSharedMeasurementParameters,
    budget: &mut Budget,
) -> Result<LcSharedMeasurementWitness, LcRelationError> {
    budget.work(1, AT)?;
    let phase = LcSharedClockPhaseParameters {
        physical: LcClockPhaseParameters {
            rate: parameters.rate.clone(),
            initial_phase: parameters.initial_phase.clone(),
            gauge: parameters.gauge.clone(),
            offset: parameters.clock_offset.clone(),
            ratio: Rational::one(),
            voltage: Rational::one(),
        },
        offset: parameters.phase_offset.clone(),
    };
    let point = cover.0.effective.witness(&phase, budget)?;
    from_point(
        cover.measurements(),
        &point,
        &parameters.measured,
        &parameters.gain,
        &parameters.energy,
        budget,
    )
}

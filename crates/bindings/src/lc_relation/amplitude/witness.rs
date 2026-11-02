// SPDX-License-Identifier: AGPL-3.0-only
//! Every reconstructed physical scale and every measured amplitude share one checked gain.

use super::{
    AT, LcAmplitudeCover, LcAmplitudeParameters, LcAmplitudeSamples, LcAmplitudeWitness,
    LcRelationError, Rational, admit, scale, storage,
};
use crate::{Budget, ExactConstant, LcPairParameters, LcQuantity as Q, LcValues, LcWitness};

/// Independent repeated supports are checked with one identical positive calibration gain.
pub(super) fn check(
    amplitude: &LcAmplitudeSamples,
    values: LcValues,
    gain: Rational,
    budget: &mut Budget,
) -> Result<LcAmplitudeWitness, LcRelationError> {
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
    let original = amplitude.relation().check_witness(values, budget)?;
    let bytes = amplitude
        .retained_payload_bytes()
        .saturating_add(amplitude.timing().retained_payload_bytes())
        .saturating_add(amplitude.relation().retained_payload_bytes())
        .saturating_add(original.retained_payload_bytes())
        .saturating_add(storage::rational(&gain))
        .saturating_add(512);
    storage::retained(bytes, budget)?;
    Ok(LcAmplitudeWitness {
        amplitude: amplitude.clone(),
        original,
        gain,
        usage: [before, budget.usage()],
    })
}

/// Phase coordinates remain original; only exact positive physical scale coordinates change.
pub(super) fn from_point(
    amplitude: &LcAmplitudeSamples,
    point: &LcWitness,
    measured: &Rational,
    gain: &Rational,
    energy: &Rational,
    budget: &mut Budget,
) -> Result<LcAmplitudeWitness, LcRelationError> {
    admit::contains(amplitude.measured(), measured, budget)?;
    admit::contains(amplitude.gain(), gain, budget)?;
    admit::positive(energy, budget)?;
    if let Some(range) = amplitude.energy() {
        admit::contains(range, energy, budget)?;
    }
    let mut values = point.values().clone();
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
    check(amplitude, values, gain.clone(), budget)
}

/// The full joint domain is parameterized explicitly, so no marginal product supplies a witness.
pub(super) fn parameters(
    cover: &LcAmplitudeCover,
    parameters: &LcAmplitudeParameters,
    budget: &mut Budget,
) -> Result<LcAmplitudeWitness, LcRelationError> {
    budget.work(1, AT)?;
    let phase = LcPairParameters {
        rate: parameters.rate.clone(),
        initial_phase: parameters.initial_phase.clone(),
        gauge: parameters.gauge.clone(),
        ratio: Rational::one(),
        voltage: Rational::one(),
    };
    let point = cover.0.effective.witness(&phase, budget)?;
    from_point(
        cover.amplitude(),
        &point,
        &parameters.measured,
        &parameters.gain,
        &parameters.energy,
        budget,
    )
}

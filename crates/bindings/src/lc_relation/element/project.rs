// SPDX-License-Identifier: AGPL-3.0-only
//! Positive inductance maps preserve rate/phase fibres and the original calibration.

use super::{
    AT, Cover, LcElementCover, LcElementParameters, LcElementSamples, LcElementWitness, LcInterval,
    LcRelationError, Q, calibration, storage,
};
use crate::{Budget, LcPairParameters, LcRelation, LcSampleTimes, LcTimingCover};
use nrz_cas::{Rational, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Only the selected element is eliminated, after admission retained its entire exact domain.
pub(super) fn cover(
    element: &LcElementSamples,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcElementCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(element.retained_payload_bytes().saturating_mul(2), budget)?;
    let mut inputs = Vec::new();
    for input in element.relation().inputs() {
        budget.work(1, AT)?;
        if input.quantity() != element.element() {
            inputs.push(input.clone());
        }
    }
    let relation = LcRelation::new(
        element.relation().epoch().into(),
        element.relation().observations(),
        inputs,
        budget,
    )?;
    let timing = LcSampleTimes::new(relation, element.timing().nominal_times().to_vec(), budget)?;
    let effective = timing.pairs_cover(cycles, pi, budget)?;
    let rate = effective.rate()[0].clone();
    let phase = effective.phase_at_rate(&rate, budget)?;
    let value = element.values()[0].clone();
    let gain = element.0.calibration.gain_at_value(&value, budget)?[0].clone();
    let parameters = LcElementParameters {
        rate,
        initial_phase: phase[0].clone(),
        gauge: Rational::zero(),
        element: value,
        calibration: gain,
        voltage: Rational::one(),
    };
    let witness = witness(element, &effective, &parameters, budget)?;
    let bytes = element
        .retained_payload_bytes()
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.calibration()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcElementCover(Rc::new(Cover {
        element: element.clone(),
        effective,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}

/// Positive rate and inductance give `R=a*L`; the original physical tuple is checked afresh.
pub(super) fn witness(
    element: &LcElementSamples,
    effective: &LcTimingCover,
    parameters: &LcElementParameters,
    budget: &mut Budget,
) -> Result<LcElementWitness, LcRelationError> {
    budget.work(1, AT)?;
    calibration::contains(element.values(), &parameters.element, budget)?;
    element.0.calibration.check(
        &parameters.element.clone().into(),
        &parameters.calibration,
        budget,
    )?;
    let ratio = parameters.rate.mul(&parameters.element, budget)?;
    let point = effective.witness(
        &LcPairParameters {
            rate: parameters.rate.clone(),
            initial_phase: parameters.initial_phase.clone(),
            gauge: parameters.gauge.clone(),
            ratio,
            voltage: parameters.voltage.clone(),
        },
        budget,
    )?;
    element.check_witness(
        point.values().clone(),
        parameters.calibration.clone(),
        budget,
    )
}

/// Monotone positive maps attain exact extrema on the retained rate/inductance rectangle.
pub(super) fn bounds(
    cover: &LcElementCover,
    quantity: Q,
    budget: &mut Budget,
) -> Result<LcInterval, LcRelationError> {
    budget.work(1, AT)?;
    let element = cover.element().values();
    let range = match quantity {
        Q::Inductance => element.clone(),
        Q::Capacitance => [
            Rational::one().div(
                &cover.rate()[1].pow(2, budget)?.mul(&element[1], budget)?,
                budget,
            )?,
            Rational::one().div(
                &cover.rate()[0].pow(2, budget)?.mul(&element[0], budget)?,
                budget,
            )?,
        ],
        Q::AmplitudeRatio => [
            cover.rate()[0].mul(&element[0], budget)?,
            cover.rate()[1].mul(&element[1], budget)?,
        ],
        Q::VoltageAmplitude | Q::CurrentAmplitude | Q::Energy => {
            return LcInterval::new(Some(Rational::zero().into()), None, budget);
        }
        _ => return cover.0.effective.bounds(quantity, budget),
    };
    calibration::interval(&range, budget)
}

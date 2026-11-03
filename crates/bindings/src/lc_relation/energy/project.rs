// SPDX-License-Identifier: AGPL-3.0-only
//! Exact energy elimination delegates physical scales while retaining the original joint owner.

use super::{AT, Cover, LcEnergyCover, LcEnergySamples, LcRelationError, calibration, storage};
use crate::{Budget, LcAmplitudeSamples, LcInput, LcQuantity as Q, LcRelation, LcSampleTimes};
use nrz_cas::{Rational, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Every feasible energy has a nonempty retained calibration fibre and vice versa.
pub(super) fn cover(
    energy: &LcEnergySamples,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcEnergyCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(energy.retained_payload_bytes().saturating_mul(2), budget)?;
    let mut inputs = energy.relation().inputs().to_vec();
    inputs.push(LcInput::new(
        Q::Energy,
        &Q::Energy.descriptor()?,
        calibration::interval(energy.energy(), budget)?,
        budget,
    )?);
    let relation = LcRelation::new(
        energy.relation().epoch().into(),
        energy.relation().observations(),
        inputs,
        budget,
    )?;
    let amplitude = energy.amplitude();
    let timing = LcSampleTimes::new(
        relation,
        amplitude.timing().nominal_times().to_vec(),
        budget,
    )?;
    let effective = LcAmplitudeSamples::new(
        timing,
        amplitude.channel(),
        amplitude.samples().to_vec(),
        &calibration::interval(amplitude.gain(), budget)?,
        budget,
    )?
    .pairs_cover(cycles, pi, budget)?;
    let point = effective.feasibility_witness();
    let value = point
        .original()
        .values()
        .energy
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let gain = energy.0.calibration.gain_at_value(value, budget)?[0].clone();
    let witness = energy.check_witness(
        point.original().values().clone(),
        point.gain().clone(),
        gain,
        budget,
    )?;
    let bytes = energy
        .retained_payload_bytes()
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.original().gain()))
        .saturating_add(storage::rational(witness.calibration()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcEnergyCover(Rc::new(Cover {
        energy: energy.clone(),
        effective,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}

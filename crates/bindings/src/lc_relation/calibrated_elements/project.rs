// SPDX-License-Identifier: AGPL-3.0-only
//! A redundant original-implied L strip has a complete converse through its retained gain fibre.

use super::{
    AT, Cover, LcCalibratedElements, LcCalibratedElementsCover, LcInput, LcRelationError, Q,
    calibration, storage,
};
use crate::{Budget, LcRelation, LcSampleTimes};
use nrz_cas::{Rational, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Every original joint tuple satisfies the implied strip, and each strip point has a common gain.
pub(super) fn cover(
    question: &LcCalibratedElements,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcCalibratedElementsCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(
        question
            .retained_payload_bytes()
            .saturating_mul(2)
            .saturating_add(8_192),
        budget,
    )?;
    let mut inputs = question.relation().inputs().to_vec();
    inputs.push(LcInput::new(
        Q::Inductance,
        &Q::Inductance.descriptor()?,
        calibration::interval(question.inductance(), budget)?,
        budget,
    )?);
    let relation = LcRelation::new(
        question.relation().epoch().into(),
        question.relation().observations(),
        inputs,
        budget,
    )?;
    let timing = LcSampleTimes::new(relation, question.timing().nominal_times().to_vec(), budget)?;
    let effective = timing.joint_elements_cover(cycles, pi, budget)?;
    let physical = effective.feasibility_witness();
    let inductance = physical
        .values()
        .inductance
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let fibre = question.calibration_at_inductance(inductance, budget)?;
    let witness = question.restore_physical(physical, fibre[0].clone(), budget)?;
    let bytes = question
        .retained_payload_bytes()
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.calibration()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcCalibratedElementsCover(Rc::new(Cover {
        question: question.clone(),
        effective,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}

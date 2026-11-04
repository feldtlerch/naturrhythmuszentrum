// SPDX-License-Identifier: AGPL-3.0-only
//! Independently implied positive rate bounds retain all original element restrictions and cycle aliases.

use super::{AT, LcRelationError, calibration, storage};
use crate::{
    Budget, ExactConstant, LcInput, LcQuantity as Q, LcRelation, LcSampleTimes, LcTimingCover,
};
use nrz_cas::{Rational, pi_bounds::PiBoundCertificate};

/// Positive independent element boxes project exactly to their product interval before timing intersection.
pub(super) fn domain(
    timing: &LcSampleTimes,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<([[Rational; 2]; 2], LcTimingCover), LcRelationError> {
    budget.work(1, AT)?;
    storage::retained(
        timing
            .relation()
            .retained_payload_bytes()
            .saturating_mul(3)
            .saturating_add(8_192),
        budget,
    )?;
    let mut elements = [None, None];
    let mut inputs = Vec::new();
    for input in timing.relation().inputs() {
        budget.work(1, AT)?;
        let slot = match input.quantity() {
            Q::Inductance => Some(0),
            Q::Capacitance => Some(1),
            Q::Time(_) | Q::Phase(_) | Q::InitialPhase | Q::Rate | Q::AngularFrequency => None,
            _ => return Err(LcRelationError::Projection),
        };
        if let Some(slot) = slot {
            let bounds = calibration::positive_bounds(input.interval(), budget)?;
            if let Some(previous) = &mut elements[slot] {
                calibration::intersect(previous, &bounds, budget)?;
            } else {
                elements[slot] = Some(bounds);
            }
        } else {
            inputs.push(input.clone());
        }
    }
    let [inductance, capacitance] = elements;
    let elements = [
        inductance.ok_or(LcRelationError::Projection)?,
        capacitance.ok_or(LcRelationError::Projection)?,
    ];
    let rate = [
        inverse_root(elements[0][1].mul(&elements[1][1], budget)?, budget)?,
        inverse_root(elements[0][0].mul(&elements[1][0], budget)?, budget)?,
    ];
    inputs.push(LcInput::new(
        Q::Rate,
        &Q::Rate.descriptor()?,
        calibration::interval(&rate, budget)?,
        budget,
    )?);
    let relation = LcRelation::new(
        timing.relation().epoch().into(),
        timing.relation().observations(),
        inputs,
        budget,
    )?;
    let timing = LcSampleTimes::new(relation, timing.nominal_times().to_vec(), budget)?;
    let cover = timing.pairs_cover(cycles, pi, budget)?;
    Ok((elements, cover))
}

/// The closed rational endpoint adapter refuses an irrational normalized product root.
fn inverse_root(product: Rational, budget: &mut Budget) -> Result<Rational, LcRelationError> {
    let root = ExactConstant::from(product)
        .positive()
        .ok_or(LcRelationError::Positivity)?
        .sqrt(budget)?;
    let value = root.as_rational().ok_or(LcRelationError::Projection)?;
    Ok(Rational::one().div(value, budget)?)
}

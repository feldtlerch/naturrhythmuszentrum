// SPDX-License-Identifier: AGPL-3.0-only
//! Only independent original energy is eliminated; the full physical question stays retained.

use super::{
    AT, LcQuantity, LcRelation, LcRelationError, LcSampleTimes, LcTimingCover, calibration, storage,
};
use crate::Budget;
use nrz_cas::{Rational, pi_bounds::PiBoundCertificate};

/// Retain positive repeated energy intersections and recover the complete auxiliary timing domain.
pub(super) fn domain(
    timing: &LcSampleTimes,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<([Rational; 2], LcTimingCover), LcRelationError> {
    budget.work(1, AT)?;
    storage::retained(
        timing
            .relation()
            .retained_payload_bytes()
            .saturating_mul(2)
            .saturating_add(2_048),
        budget,
    )?;
    let mut energy = None;
    let mut inputs = Vec::new();
    for input in timing.relation().inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            LcQuantity::Energy => {
                let next = calibration::positive_bounds(input.interval(), budget)?;
                if let Some(previous) = &mut energy {
                    calibration::intersect(previous, &next, budget)?;
                } else {
                    energy = Some(next);
                }
            }
            LcQuantity::Time(_)
            | LcQuantity::Phase(_)
            | LcQuantity::InitialPhase
            | LcQuantity::Rate
            | LcQuantity::AngularFrequency => inputs.push(input.clone()),
            _ => return Err(LcRelationError::Projection),
        }
    }
    let energy = energy.ok_or(LcRelationError::Projection)?;
    let relation = LcRelation::new(
        timing.relation().epoch().into(),
        timing.relation().observations(),
        inputs,
        budget,
    )?;
    let independent = LcSampleTimes::new(relation, timing.nominal_times().to_vec(), budget)?;
    let effective = independent.pairs_cover(cycles, pi, budget)?;
    Ok((energy, effective))
}

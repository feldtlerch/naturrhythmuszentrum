// SPDX-License-Identifier: AGPL-3.0-only
//! Shared offset faces and uncertain sample-time faces are eliminated in one joint domain.

use super::{
    AT, Cover, LcRelation, LcRelationError, LcSharedTiming, LcSharedTimingCover, Rational, storage,
    witness,
};
use crate::{Budget, LcInput, LcQuantity as Q, LcSampleTimes};
use nrz_cas::pi_bounds::PiBoundCertificate;
use std::rc::Rc;

/// Exact timing elimination retains `beta=a*offset` and every independent sample error.
pub(super) fn cover(
    timing: &LcSharedTiming,
    mut cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcSharedTimingCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    if cycles.len() != timing.relation().observations() {
        return Err(LcRelationError::Observation);
    }
    let relation = effective(timing, budget)?;
    let mut nominal = timing.nominal_times().to_vec();
    for offset in timing.offset() {
        nominal.push(Rational::zero().sub(offset, budget)?);
    }
    let independent = LcSampleTimes::new(relation, nominal, budget)?;
    cycles.extend([Rational::zero(), Rational::zero()]);
    let effective = independent.pairs_cover(cycles, pi, budget)?;
    let point = effective.feasibility_witness();
    let rate = point
        .values()
        .rate
        .as_rational()
        .ok_or(LcRelationError::Projection)?;
    let beta = point
        .values()
        .initial_phase
        .as_ref()
        .and_then(crate::ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    let offset = beta.div(rate, budget)?;
    let witness = witness::from_point(timing, point, &offset, budget)?;
    let bytes = timing
        .retained_payload_bytes()
        .saturating_add(timing.relation().retained_payload_bytes())
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.offset()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcSharedTimingCover(Rc::new(Cover {
        timing: timing.clone(),
        effective,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}

/// Physical starting phase zero becomes a correlated effective phase, never an independent box.
fn effective(timing: &LcSharedTiming, budget: &mut Budget) -> Result<LcRelation, LcRelationError> {
    let count = timing.relation().observations();
    let total = count.checked_add(2).ok_or(LcRelationError::Capacity)?;
    storage::retained(
        timing
            .relation()
            .retained_payload_bytes()
            .saturating_mul(2)
            .saturating_add(32_768),
        budget,
    )?;
    let mut inputs = Vec::new();
    for input in timing.relation().inputs() {
        budget.work(1, AT)?;
        if !matches!(input.quantity(), Q::Time(_) | Q::InitialPhase) {
            inputs.push(input.clone());
        }
    }
    for (index, (nominal, error)) in timing
        .nominal_times()
        .iter()
        .zip(timing.errors())
        .enumerate()
    {
        let range = [
            nominal.add(&error[0], budget)?,
            nominal.add(&error[1], budget)?,
        ];
        inputs.push(super::super::admit::input(Q::Time(index), range, budget)?);
    }
    offset_faces(timing, &mut inputs, budget)?;
    LcRelation::new(timing.relation().epoch().into(), total, inputs, budget)
}

/// Redundant finite opposite faces preserve exactly `a*d_min<=beta<=a*d_max`.
fn offset_faces(
    timing: &LcSharedTiming,
    inputs: &mut Vec<LcInput>,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let rate = &timing.0.rate;
    let offset = timing.offset();
    let range = super::super::admit::product(rate, offset, budget)?;
    inputs.push(super::super::admit::input(Q::InitialPhase, range, budget)?);
    let span = rate[1].mul(&offset[1].sub(&offset[0], budget)?, budget)?;
    let negative_span = Rational::zero().sub(&span, budget)?;
    let count = timing.relation().observations();
    for (index, offset, phase) in [
        (count, &offset[0], [Rational::zero(), span]),
        (count + 1, &offset[1], [negative_span, Rational::zero()]),
    ] {
        let time = Rational::zero().sub(offset, budget)?;
        inputs.push(super::super::admit::input(
            Q::Time(index),
            [time.clone(), time],
            budget,
        )?);
        inputs.push(super::super::admit::input(Q::Phase(index), phase, budget)?);
    }
    Ok(())
}

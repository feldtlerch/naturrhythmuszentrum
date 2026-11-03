// SPDX-License-Identifier: AGPL-3.0-only
//! Shared offset faces and uncertain sample-time faces are eliminated in one joint domain.

pub(super) mod restricted;

use super::{
    AT, Cover, LcRelation, LcRelationError, LcSharedTiming, LcSharedTimingCover, Rational, storage,
    witness,
};
use crate::{Budget, LcInput, LcQuantity as Q, LcSampleTimes};
use nrz_cas::pi_bounds::PiBoundCertificate;
use std::rc::Rc;

/// Exact timing elimination retains `beta=alpha+a*offset` and every independent sample error.
pub(super) fn cover(
    timing: &LcSharedTiming,
    mut cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcSharedTimingCover, LcRelationError> {
    if timing.0.narrow {
        return restricted::cover(timing, cycles, pi, budget);
    }
    let before = budget.usage();
    budget.work(1, AT)?;
    if cycles.len() != timing.relation().observations() {
        return Err(LcRelationError::Observation);
    }
    let relation = effective(timing, budget)?;
    let mut nominal = timing.nominal_times().to_vec();
    if timing.initial_phase_support().is_some() {
        for offset in timing.offset() {
            nominal.push(Rational::zero().sub(offset, budget)?);
        }
        cycles.extend([Rational::zero(), Rational::zero()]);
    }
    let independent = LcSampleTimes::new(relation, nominal, budget)?;
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
    let mut offset = timing.offset().clone();
    if let Some([lower, upper]) = timing.initial_phase_support() {
        let image = [
            beta.sub(upper, budget)?.div(rate, budget)?,
            beta.sub(lower, budget)?.div(rate, budget)?,
        ];
        crate::lc_relation::calibration::intersect(&mut offset, &image, budget)?;
    }
    let witness = witness::from_point(timing, point, &offset[0], budget)?;
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
        measured: None,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}

/// Physical initial phase and shared offset remain correlated through the effective intercept.
fn effective(timing: &LcSharedTiming, budget: &mut Budget) -> Result<LcRelation, LcRelationError> {
    let count = timing.relation().observations();
    let auxiliary = if timing.initial_phase_support().is_some() {
        2
    } else {
        0
    };
    let total = count
        .checked_add(auxiliary)
        .ok_or(LcRelationError::Capacity)?;
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
    if let Some(initial) = timing.initial_phase_support() {
        offset_faces(timing, initial, &mut inputs, budget)?;
    }
    LcRelation::new(timing.relation().epoch().into(), total, inputs, budget)
}

/// Redundant finite opposite faces preserve exactly `A_min+a*d_min<=beta<=A_max+a*d_max`.
fn offset_faces(
    timing: &LcSharedTiming,
    initial: &[Rational; 2],
    inputs: &mut Vec<LcInput>,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let rate = &timing.0.rate;
    let offset = timing.offset();
    let product = super::super::admit::product(rate, offset, budget)?;
    let range = [
        initial[0].add(&product[0], budget)?,
        initial[1].add(&product[1], budget)?,
    ];
    inputs.push(super::super::admit::input(Q::InitialPhase, range, budget)?);
    let span = rate[1].mul(&offset[1].sub(&offset[0], budget)?, budget)?;
    let count = timing.relation().observations();
    for (index, offset, phase) in [
        (
            count,
            &offset[0],
            [initial[0].clone(), initial[1].add(&span, budget)?],
        ),
        (
            count + 1,
            &offset[1],
            [initial[0].sub(&span, budget)?, initial[1].clone()],
        ),
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

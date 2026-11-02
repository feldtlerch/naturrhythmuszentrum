// SPDX-License-Identifier: AGPL-3.0-only
//! A primitive timestamp lattice proves every relative cycle family without a frequency cutoff.

use super::super::{Bounds, support};
use super::{LcAliasLattice, LcRelation, LcRelationError, Rational};
use crate::{Budget, ExactConstant, LcInput, LcInterval, LcQuantity};
use nrz_cas::pi_bounds::CheckedPiBounds;
use std::cmp::Ordering;

/// Complete mathematical reduction to a bounded fundamental representative family.
#[derive(Debug)]
pub(super) struct Fundamental {
    /// Original constraints plus a derived representative-cell bound, not a physical prior.
    pub(super) relation: LcRelation,
    /// Exact integer timestamps in units of the primitive quantum.
    pub(super) ticks: [Rational; 3],
}

/// Read exact timestamps and lifted phase intervals, preserving repeated declarations.
fn observations(
    relation: &LcRelation,
    cycles: &[Rational],
    budget: &mut Budget,
) -> Result<[(Rational, Bounds); 3], LcRelationError> {
    if relation.observations() != 3 || cycles.len() != 3 {
        return Err(LcRelationError::Observation);
    }
    let mut times: [Option<Bounds>; 3] = [None, None, None];
    let mut phases: [Option<Bounds>; 3] = [None, None, None];
    let turn = support::turn(budget)?;
    for cycle in cycles {
        support::integer(cycle, budget)?;
    }
    for input in relation.inputs() {
        match input.quantity() {
            LcQuantity::Time(i) => support::constrain(
                &mut times[i],
                support::bounds(input.interval(), budget)?,
                budget,
            )?,
            LcQuantity::Phase(i) => {
                let shift = turn.mul(&cycles[i].clone().into(), budget)?;
                support::constrain(
                    &mut phases[i],
                    support::lifted(input.interval(), &shift, budget)?,
                    budget,
                )?;
            }
            LcQuantity::InitialPhase => {}
            _ => return Err(LcRelationError::Projection),
        }
    }
    let mut result = Vec::with_capacity(3);
    for (time, phase) in times.into_iter().zip(phases) {
        let time = time.ok_or(LcRelationError::Projection)?;
        if time[0] != time[1] {
            return Err(LcRelationError::Projection);
        }
        result.push((time[0].clone(), phase.ok_or(LcRelationError::Projection)?));
    }
    result.try_into().map_err(|_| LcRelationError::Observation)
}

/// Integer timestamps and a checked Bezout identity establish the primitive cycle step.
fn lattice(
    observations: &[(Rational, Bounds); 3],
    lattice: &LcAliasLattice,
    budget: &mut Budget,
) -> Result<([Rational; 3], [Rational; 2]), LcRelationError> {
    if lattice.quantum.is_negative() || lattice.quantum.is_zero() {
        return Err(LcRelationError::Projection);
    }
    let mut ticks = [Rational::zero(), Rational::zero(), Rational::zero()];
    for (tick, (time, _)) in ticks.iter_mut().zip(observations) {
        *tick = time.div(&lattice.quantum, budget)?;
        support::integer(tick, budget)?;
    }
    let gaps = [
        ticks[1].sub(&ticks[0], budget)?,
        ticks[2].sub(&ticks[0], budget)?,
    ];
    if gaps[0].is_negative()
        || gaps[0].is_zero()
        || gaps[0].compare(&gaps[1], budget)? != Ordering::Less
    {
        return Err(LcRelationError::Projection);
    }
    for coefficient in &lattice.bezout {
        support::integer(coefficient, budget)?;
    }
    let identity = gaps[0]
        .mul(&lattice.bezout[0], budget)?
        .add(&gaps[1].mul(&lattice.bezout[1], budget)?, budget)?;
    if identity != Rational::one() {
        return Err(LcRelationError::CycleCover);
    }
    Ok((ticks, gaps))
}

/// Only integer zero survives the residual full-turn interval, forcing all cycles onto the lattice.
fn integer_gap(
    observations: &[(Rational, Bounds); 3],
    gaps: &[Rational; 2],
    pi: &CheckedPiBounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let [middle, last] = gaps;
    let delta = last.sub(middle, budget)?;
    let phase = [&observations[0].1, &observations[1].1, &observations[2].1];
    let residual = [
        last.mul(&phase[1][0], budget)?
            .sub(&middle.mul(&phase[2][1], budget)?, budget)?
            .sub(&delta.mul(&phase[0][1], budget)?, budget)?,
        last.mul(&phase[1][1], budget)?
            .sub(&middle.mul(&phase[2][0], budget)?, budget)?
            .sub(&delta.mul(&phase[0][0], budget)?, budget)?,
    ];
    let turn = pi
        .contract()
        .lower
        .mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let negative = Rational::zero().sub(&turn, budget)?;
    let above = residual[0].compare(&negative, budget)? == Ordering::Greater;
    let below = residual[1].compare(&turn, budget)? == Ordering::Less;
    if !above || !below {
        return Err(LcRelationError::CycleCover);
    }
    Ok(())
}

/// Derive representative support and prove exactly nonnegative aliases have positive rate.
pub(super) fn check(
    relation: &LcRelation,
    cycles: &[Rational],
    proposal: &LcAliasLattice,
    pi: &CheckedPiBounds,
    budget: &mut Budget,
) -> Result<Fundamental, LcRelationError> {
    let observations = observations(relation, cycles, budget)?;
    let (ticks, gaps) = lattice(&observations, proposal, budget)?;
    integer_gap(&observations, &gaps, pi, budget)?;
    let duration = observations[2].0.sub(&observations[0].0, budget)?;
    let range = [
        observations[2].1[0]
            .sub(&observations[0].1[1], budget)?
            .div(&duration, budget)?,
        observations[2].1[1]
            .sub(&observations[0].1[0], budget)?
            .div(&duration, budget)?,
    ];
    let step_lower = pi
        .contract()
        .lower
        .mul(&Rational::from_ratio(2, 1, budget)?, budget)?
        .div(&proposal.quantum, budget)?;
    let positive = !range[0].is_negative() && !range[0].is_zero();
    let below_next = range[1].compare(&step_lower, budget)? == Ordering::Less;
    if !positive || !below_next {
        return Err(LcRelationError::CycleCover);
    }
    let mut inputs = relation.inputs().to_vec();
    inputs.push(LcInput::new(
        LcQuantity::Rate,
        &LcQuantity::Rate.descriptor()?,
        LcInterval::new(
            Some(ExactConstant::from(range[0].clone())),
            Some(range[1].clone().into()),
            budget,
        )?,
        budget,
    )?);
    let fundamental = LcRelation::new(
        relation.epoch().into(),
        relation.observations(),
        inputs,
        budget,
    )?;
    Ok(Fundamental {
        relation: fundamental,
        ticks,
    })
}

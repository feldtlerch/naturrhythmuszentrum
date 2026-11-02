// SPDX-License-Identifier: AGPL-3.0-only
//! Two finite strips encode exactly one existential positive-rate timestamp interval.

use super::{AT, Bounds, LcRelation, LcRelationError, LcSampleTimes, Rational, storage};
use crate::{Budget, ExactConstant, LcInput, LcInterval, LcQuantity as Q};
use nrz_cas::syntax::Constant;
use std::cmp::Ordering;

/// Effective original-equivalent strip inputs and retained original phase supports.
#[derive(Debug)]
pub(super) struct Prepared {
    /// Original non-timing inputs and exactly equivalent auxiliary time/phase strips.
    pub(super) relation: LcRelation,
    /// Lifted original phase intersections, without redundant widening.
    pub(super) phases: Vec<Bounds>,
    /// Auxiliary zero cycles are differences from the retained original base proposal.
    pub(super) cycles: Vec<Rational>,
    /// Original full-turn shifts prepared once for lifting and physical restoration.
    pub(super) shifts: Vec<ExactConstant>,
}

/// Original feasible tuples imply both auxiliary strips at the same rate and phase.
pub(super) fn question(
    timing: &LcSampleTimes,
    cycles: &[Rational],
    budget: &mut Budget,
) -> Result<Prepared, LcRelationError> {
    budget.work(1, AT)?;
    let count = timing.relation().observations();
    if cycles.len() != count {
        return Err(LcRelationError::Observation);
    }
    let total = count.checked_mul(2).ok_or(LcRelationError::Capacity)?;
    storage::retained(
        timing
            .relation()
            .retained_payload_bytes()
            .saturating_mul(4)
            .saturating_add(storage::size(total)?.saturating_mul(8_192)),
        budget,
    )?;
    let shifts = cycle_shifts(cycles, budget)?;
    let (mut inputs, phases) = original_inputs(timing, &shifts, budget)?;
    for (index, phase) in phases.iter().enumerate() {
        faces(timing, index, phase, &mut inputs, budget)?;
    }
    for (index, time) in timing.time_boxes().iter().enumerate() {
        for (auxiliary, endpoint) in [(2 * index, &time[1]), (2 * index + 1, &time[0])] {
            inputs.push(super::super::admit::input(
                Q::Time(auxiliary),
                [endpoint.clone(), endpoint.clone()],
                budget,
            )?);
        }
    }
    let relation = LcRelation::new(timing.relation().epoch().into(), total, inputs, budget)?;
    Ok(Prepared {
        relation,
        phases,
        cycles: vec![Rational::zero(); total],
        shifts,
    })
}

/// Each exact original cycle shift is prepared once, without rebuilding unit full turns.
fn cycle_shifts(
    cycles: &[Rational],
    budget: &mut Budget,
) -> Result<Vec<ExactConstant>, LcRelationError> {
    let one = Rational::one();
    let mut turn = None;
    let mut shifts = Vec::with_capacity(cycles.len());
    for cycle in cycles {
        budget.work(1, AT)?;
        cycle.check(budget, AT)?;
        if !cycle.is_integer() {
            return Err(LcRelationError::Cycle);
        }
        let shift = if cycle.is_zero() {
            ExactConstant::zero()
        } else {
            if turn.is_none() {
                turn = Some(
                    ExactConstant::named(Constant::Pi, budget)?
                        .mul(&Rational::from_ratio(2, 1, budget)?.into(), budget)?,
                );
            }
            let turn = turn.as_ref().ok_or(LcRelationError::Cycle)?;
            if cycle.compare(&one, budget)? == Ordering::Equal {
                turn.clone()
            } else {
                turn.mul(&cycle.clone().into(), budget)?
            }
        };
        shifts.push(shift);
    }
    Ok(shifts)
}

/// Each original phase restriction contributes both exact faces and its reconstruction box.
fn original_inputs(
    timing: &LcSampleTimes,
    shifts: &[ExactConstant],
    budget: &mut Budget,
) -> Result<(Vec<LcInput>, Vec<Bounds>), LcRelationError> {
    let mut phases = vec![None; timing.relation().observations()];
    let mut inputs = Vec::new();
    for input in timing.relation().inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            Q::Time(_) => (),
            Q::Phase(index) => {
                let shift = &shifts[index];
                let interval = input.interval();
                let lifted = LcInterval::new(
                    Some(
                        interval
                            .lower()
                            .ok_or(LcRelationError::Projection)?
                            .add(shift, budget)?,
                    ),
                    Some(
                        interval
                            .upper()
                            .ok_or(LcRelationError::Projection)?
                            .add(shift, budget)?,
                    ),
                    budget,
                )?;
                intersect(
                    &mut phases[index],
                    super::super::admit::bounds(&lifted, budget)?,
                    budget,
                )?;
            }
            _ => inputs.push(input.clone()),
        }
    }
    let phases = phases
        .into_iter()
        .map(|phase| phase.ok_or(LcRelationError::Projection))
        .collect::<Result<_, _>>()?;
    Ok((inputs, phases))
}

/// Opposite faces are redundant because `0<a<=a_max` and the original time width is nonnegative.
fn faces(
    timing: &LcSampleTimes,
    index: usize,
    phase: &Bounds,
    inputs: &mut Vec<LcInput>,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let time = &timing.time_boxes()[index];
    let width = time[1].sub(&time[0], budget)?;
    let span = timing.rate_support()[1].mul(&width, budget)?;
    let [lower, upper] = phase;
    for (auxiliary, range) in [
        (2 * index, [lower.clone(), upper.add(&span, budget)?]),
        (2 * index + 1, [lower.sub(&span, budget)?, upper.clone()]),
    ] {
        inputs.push(super::super::admit::input(
            Q::Phase(auxiliary),
            range,
            budget,
        )?);
    }
    Ok(())
}

/// Original repeated phase declarations intersect before timestamp reconstruction.
fn intersect(
    slot: &mut Option<Bounds>,
    mut next: Bounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    if let Some(previous) = slot {
        if previous[0].compare(&next[0], budget)? == Ordering::Greater {
            next[0].clone_from(&previous[0]);
        }
        if previous[1].compare(&next[1], budget)? == Ordering::Less {
            next[1].clone_from(&previous[1]);
        }
    }
    if next[0].compare(&next[1], budget)? == Ordering::Greater {
        return Err(LcRelationError::Containment);
    }
    *slot = Some(next);
    Ok(())
}

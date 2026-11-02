// SPDX-License-Identifier: AGPL-3.0-only
//! Shared phase admission retains every original physical phase and initial-phase constraint.

use super::{AT, Bounds, LcInterval, LcRelation, LcRelationError, Rational};
use crate::{Budget, ExactConstant, LcQuantity as Q};
use std::cmp::Ordering;

/// Finite rational calibration coordinates retain exact arithmetic and caller resources.
pub(super) fn bounds(
    interval: &LcInterval,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    budget.work(1, AT)?;
    let lower = interval
        .lower()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    let upper = interval
        .upper()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    lower.check(budget, AT)?;
    upper.check(budget, AT)?;
    Ok([lower.clone(), upper.clone()])
}

/// Original total phase restrictions must contain the complete declared measurement image.
pub(super) fn question(
    relation: &LcRelation,
    samples: &[LcInterval],
    offset: &Bounds,
    budget: &mut Budget,
) -> Result<Option<Bounds>, LcRelationError> {
    budget.work(1, AT)?;
    if samples.len() != relation.observations() {
        return Err(LcRelationError::Observation);
    }
    for sample in samples {
        budget.work(1, AT)?;
        if sample.lower().is_none() || sample.upper().is_none() {
            return Err(LcRelationError::Projection);
        }
    }
    let mut initial = None;
    let exact_offset = [
        ExactConstant::from(offset[0].clone()),
        ExactConstant::from(offset[1].clone()),
    ];
    for input in relation.inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            Q::InitialPhase => {
                let next = bounds(input.interval(), budget)?;
                if let Some(previous) = &mut initial {
                    intersect(previous, &next, budget)?;
                } else {
                    initial = Some(next);
                }
            }
            Q::Phase(index) => image(input.interval(), &samples[index], &exact_offset, budget)?,
            _ => (),
        }
    }
    Ok(initial)
}

/// Both endpoint images imply containment of the whole additive interval image.
fn image(
    original: &LcInterval,
    sample: &LcInterval,
    offset: &[ExactConstant; 2],
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let lower = sample
        .lower()
        .ok_or(LcRelationError::Projection)?
        .add(&offset[0], budget)?;
    let upper = sample
        .upper()
        .ok_or(LcRelationError::Projection)?
        .add(&offset[1], budget)?;
    let lower_inside = original.contains(&lower, budget)?;
    let upper_inside = original.contains(&upper, budget)?;
    if !lower_inside || !upper_inside {
        return Err(LcRelationError::Containment);
    }
    Ok(())
}

/// Joint calibration fibres retain exact intersections, including repeated declarations.
pub(super) fn intersect(
    left: &mut Bounds,
    right: &Bounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    if right[0].compare(&left[0], budget)? == Ordering::Greater {
        left[0].clone_from(&right[0]);
    }
    if right[1].compare(&left[1], budget)? == Ordering::Less {
        left[1].clone_from(&right[1]);
    }
    if left[0].compare(&left[1], budget)? == Ordering::Greater {
        return Err(LcRelationError::Containment);
    }
    Ok(())
}

/// Membership is a checked constraint, not an inference from coincident marginal centers.
pub(super) fn contains(
    range: &Bounds,
    value: &Rational,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    value.check(budget, AT)?;
    let above = value.compare(&range[0], budget)? != Ordering::Less;
    let below = value.compare(&range[1], budget)? != Ordering::Greater;
    if !above || !below {
        return Err(LcRelationError::Containment);
    }
    Ok(())
}

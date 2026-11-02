// SPDX-License-Identifier: AGPL-3.0-only
//! Shared-clock admission checks every original timing constraint and physical clock premise.

use super::{AT, LcInterval, LcRelation, LcRelationError, Rational};
use crate::{Budget, ExactConstant, LcQuantity as Q};
use std::cmp::Ordering;

/// Finite rational supports are required by the closed clock-coordinate elimination.
pub(super) fn bounds(
    interval: &LcInterval,
    budget: &mut Budget,
) -> Result<[Rational; 2], LcRelationError> {
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

/// Every original constraint stays either in the effective solver or in this implication proof.
pub(super) fn question(
    relation: &LcRelation,
    nominal: &[Rational],
    offset: &[Rational; 2],
    budget: &mut Budget,
) -> Result<[Rational; 2], LcRelationError> {
    budget.work(1, AT)?;
    if nominal.len() != relation.observations() || nominal.len() < 2 {
        return Err(LcRelationError::Observation);
    }
    for time in nominal {
        budget.work(1, AT)?;
        time.check(budget, AT)?;
    }
    let mut rate = None;
    let mut phase = None;
    for input in relation.inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            Q::Rate | Q::AngularFrequency => {
                intersect(&mut rate, bounds(input.interval(), budget)?, budget)?;
            }
            Q::InitialPhase => intersect(&mut phase, bounds(input.interval(), budget)?, budget)?,
            Q::Time(index) => {
                let image = [
                    nominal[index].add(&offset[0], budget)?,
                    nominal[index].add(&offset[1], budget)?,
                ];
                timing(input.interval(), &image, budget)?;
            }
            _ => (),
        }
    }
    let rate = rate.ok_or(LcRelationError::Projection)?;
    let phase = phase.ok_or(LcRelationError::Projection)?;
    if rate[0].is_negative() || rate[0].is_zero() {
        return Err(LcRelationError::Positivity);
    }
    if !phase[0].is_zero() || !phase[1].is_zero() {
        return Err(LcRelationError::Projection);
    }
    Ok(rate)
}

/// Repeated original declarations intersect without replacing an earlier constraint.
fn intersect(
    slot: &mut Option<[Rational; 2]>,
    mut next: [Rational; 2],
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

/// Every original timestamp support must contain the full declared shared-clock image.
fn timing(
    interval: &LcInterval,
    image: &[Rational; 2],
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    for (endpoint, expected, forbidden) in [
        (interval.lower(), &image[0], Ordering::Greater),
        (interval.upper(), &image[1], Ordering::Less),
    ] {
        if let Some(endpoint) = endpoint {
            let endpoint = endpoint.as_rational().ok_or(LcRelationError::Projection)?;
            if endpoint.compare(expected, budget)? == forbidden {
                return Err(LcRelationError::Containment);
            }
        }
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Independent time supports intersect before any multiplication or projection.

use super::{AT, Bounds, LcRelation, LcRelationError, Rational, storage};
use crate::{Budget, ExactConstant, LcInput, LcInterval, LcQuantity as Q};
use std::cmp::Ordering;

/// Retain every original time and rate constraint; other inputs remain in the later solver.
pub(super) fn question(
    relation: &LcRelation,
    nominal: &[Rational],
    budget: &mut Budget,
) -> Result<(Vec<Bounds>, Bounds), LcRelationError> {
    budget.work(1, AT)?;
    if nominal.len() != relation.observations() || nominal.len() < 2 {
        return Err(LcRelationError::Observation);
    }
    storage::retained(storage::size(nominal.len())?.saturating_mul(8_192), budget)?;
    let mut times = vec![None; nominal.len()];
    for value in nominal {
        budget.work(1, AT)?;
        value.check(budget, AT)?;
    }
    let mut rate = None;
    for input in relation.inputs() {
        budget.work(1, AT)?;
        let slot = match input.quantity() {
            Q::Time(index) => &mut times[index],
            Q::Rate | Q::AngularFrequency => &mut rate,
            _ => continue,
        };
        constrain(slot, bounds(input.interval(), budget)?, budget)?;
    }
    let rate = rate.ok_or(LcRelationError::Projection)?;
    if rate[0].is_zero() || rate[0].is_negative() {
        return Err(LcRelationError::Positivity);
    }
    let times = times
        .into_iter()
        .map(|slot| slot.ok_or(LcRelationError::Projection))
        .collect::<Result<_, _>>()?;
    Ok((times, rate))
}

/// Finite rational supports are required for exact corner multiplication.
pub(super) fn bounds(
    interval: &LcInterval,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    budget.work(1, AT)?;
    let lower = interval.lower().and_then(ExactConstant::as_rational);
    let upper = interval.upper().and_then(ExactConstant::as_rational);
    let (lower, upper) = lower.zip(upper).ok_or(LcRelationError::Projection)?;
    lower.check(budget, AT)?;
    upper.check(budget, AT)?;
    Ok([lower.clone(), upper.clone()])
}

/// Repeated declarations preserve the intersection rather than selecting the last box.
fn constrain(
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

/// Exact bilinear extrema occur at rational corners, including mixed-sign timing errors.
pub(super) fn product(
    left: &Bounds,
    right: &Bounds,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    let first = left[0].mul(&right[0], budget)?;
    let mut result = [first.clone(), first];
    for a in left {
        for b in right {
            budget.work(1, AT)?;
            let value = a.mul(b, budget)?;
            if value.compare(&result[0], budget)? == Ordering::Less {
                result[0].clone_from(&value);
            }
            if value.compare(&result[1], budget)? == Ordering::Greater {
                result[1].clone_from(&value);
            }
        }
    }
    Ok(result)
}

/// Effective coordinates retain the original coherent physical meanings.
pub(super) fn input(
    quantity: Q,
    bounds: Bounds,
    budget: &mut Budget,
) -> Result<LcInput, LcRelationError> {
    let [lower, upper] = bounds;
    let interval = LcInterval::new(Some(lower.into()), Some(upper.into()), budget)?;
    LcInput::new(quantity, &quantity.descriptor()?, interval, budget)
}

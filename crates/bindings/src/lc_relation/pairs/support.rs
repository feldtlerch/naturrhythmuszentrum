// SPDX-License-Identifier: AGPL-3.0-only
//! Exact finite rational supports, checked intersections and full-turn lifting.

use super::{AT, Bounds, LcRelationError, Rational};
use crate::{Budget, ExactConstant, LcInterval};
use nrz_cas::syntax::Constant;
use std::cmp::Ordering;

/// Every finite coordinate remains exact; unsupported algebra never becomes a float center.
pub(super) fn rational(value: &ExactConstant) -> Result<Rational, LcRelationError> {
    value
        .as_rational()
        .cloned()
        .ok_or(LcRelationError::Projection)
}

/// Lift both original phase endpoints using the same proposed mathematical integer.
pub(super) fn lifted(
    interval: &LcInterval,
    shift: &ExactConstant,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    let lower = interval.lower().ok_or(LcRelationError::Projection)?;
    let upper = interval.upper().ok_or(LcRelationError::Projection)?;
    Ok([
        rational(&lower.add(shift, budget)?)?,
        rational(&upper.add(shift, budget)?)?,
    ])
}

/// Original coherent rational bounds are the zero-lift special case.
pub(super) fn bounds(
    interval: &LcInterval,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    lifted(interval, &ExactConstant::zero(), budget)
}

/// Empty support is distinguished from an unsupported declaration inside this adapter.
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

/// Repeated declarations retain intersection semantics and their original source record.
pub(super) fn constrain(
    slot: &mut Option<Bounds>,
    next: Bounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    if let Some(previous) = slot {
        intersect(previous, &next, budget)?;
    } else {
        *slot = Some(next);
    }
    Ok(())
}

/// Original exact full-turn constant, used only as mathematical syntax.
pub(super) fn turn(budget: &mut Budget) -> Result<ExactConstant, LcRelationError> {
    Ok(ExactConstant::named(Constant::Pi, budget)?
        .mul(&Rational::from_ratio(2, 1, budget)?.into(), budget)?)
}

/// A branch identifier never accepts a fractional or rounded cycle.
pub(super) fn integer(value: &Rational, budget: &mut Budget) -> Result<(), LcRelationError> {
    let (numerator, denominator) = value.component_bits();
    let work = numerator
        .div_ceil(64)
        .saturating_add(denominator.div_ceil(64));
    budget.work(work.max(1), AT)?;
    value.check(budget, AT)?;
    if !value.is_integer() {
        return Err(LcRelationError::Cycle);
    }
    Ok(())
}

/// Exact membership is checked with the same caller arithmetic allowance.
pub(super) fn contains(
    range: &Bounds,
    value: &Rational,
    budget: &mut Budget,
) -> Result<bool, LcRelationError> {
    let lower = value.compare(&range[0], budget)? != Ordering::Less;
    let upper = value.compare(&range[1], budget)? != Ordering::Greater;
    Ok(lower && upper)
}

/// Positive inversion reverses the complete rate support, retaining its common parameter.
pub(super) fn inverse_power(
    range: &Bounds,
    power: i64,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    Ok([
        Rational::one().div(&range[1].pow(power, budget)?, budget)?,
        Rational::one().div(&range[0].pow(power, budget)?, budget)?,
    ])
}

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

/// Original positive rate and optional bounded physical initial-phase premises.
#[derive(Debug)]
pub(super) struct Premises {
    /// Positive original rate interval.
    pub(super) rate: [Rational; 2],
    /// Shared offset intersection implied by all original timestamp supports.
    pub(super) offset: [Rational; 2],
    /// Repeated original initial-phase intersection; absence retains every integer gauge.
    pub(super) initial: Option<[Rational; 2]>,
}

/// Every original constraint stays either in the effective solver or in this implication proof.
pub(super) fn question(
    relation: &LcRelation,
    nominal: &[Rational],
    offset: &[Rational; 2],
    budget: &mut Budget,
) -> Result<Premises, LcRelationError> {
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
    let mut support = offset.clone();
    for input in relation.inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            Q::Rate | Q::AngularFrequency => {
                intersect(&mut rate, bounds(input.interval(), budget)?, budget)?;
            }
            Q::InitialPhase => intersect(&mut phase, bounds(input.interval(), budget)?, budget)?,
            Q::Time(index) => {
                let mut image = [
                    nominal[index].add(&support[0], budget)?,
                    nominal[index].add(&support[1], budget)?,
                ];
                crate::lc_relation::calibration::clip_interval(
                    &mut image,
                    input.interval(),
                    budget,
                )?;
                support = [
                    image[0].sub(&nominal[index], budget)?,
                    image[1].sub(&nominal[index], budget)?,
                ];
            }
            _ => (),
        }
    }
    let rate = rate.ok_or(LcRelationError::Projection)?;
    if rate[0].is_negative() || rate[0].is_zero() {
        return Err(LcRelationError::Positivity);
    }
    Ok(Premises {
        rate,
        offset: support,
        initial: phase,
    })
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

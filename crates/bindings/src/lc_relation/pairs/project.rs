// SPDX-License-Identifier: AGPL-3.0-only
//! Complete one-dimensional elimination of a convex family of phase/time strips.

use super::{AT, Bounds, LcRelationError, Rational, Strip, support};
use crate::Budget;
use nrz_cas::pi_bounds::CheckedPiBounds;
use std::cmp::Ordering;

/// A finite frequency input excludes every other relative integer lift by strict full-turn gaps.
pub(super) fn branches(
    strips: &[Strip],
    rate: &Bounds,
    fixed_phase: bool,
    pi: &CheckedPiBounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let anchor = if fixed_phase {
        strips.last()
    } else {
        strips.first()
    }
    .ok_or(LcRelationError::Observation)?;
    let turn_lower = pi
        .contract()
        .lower
        .mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let negative_turn = Rational::zero().sub(&turn_lower, budget)?;
    for strip in strips {
        budget.work(1, AT)?;
        let difference = strip.time.sub(&anchor.time, budget)?;
        let mut predicted = [
            rate[0].mul(&difference, budget)?,
            rate[1].mul(&difference, budget)?,
        ];
        if difference.is_negative() {
            predicted.swap(0, 1);
        }
        let residual = [
            predicted[0]
                .sub(&strip.phase[1], budget)?
                .add(&anchor.phase[0], budget)?,
            predicted[1]
                .sub(&strip.phase[0], budget)?
                .add(&anchor.phase[1], budget)?,
        ];
        let above = residual[0].compare(&negative_turn, budget)? == Ordering::Greater;
        let below = residual[1].compare(&turn_lower, budget)? == Ordering::Less;
        if !above || !below {
            return Err(LcRelationError::CycleCover);
        }
    }
    Ok(())
}

/// All pair differences eliminate the common starting phase without inventing independence.
pub(super) fn rate(
    strips: &[Strip],
    mut range: Bounds,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    for (i, left) in strips.iter().enumerate() {
        for right in &strips[i + 1..] {
            budget.work(1, AT)?;
            let difference = right.time.sub(&left.time, budget)?;
            let separation = [
                right.phase[0].sub(&left.phase[1], budget)?,
                right.phase[1].sub(&left.phase[0], budget)?,
            ];
            if difference.is_zero() {
                if !support::contains(&separation, &Rational::zero(), budget)? {
                    return Err(LcRelationError::Containment);
                }
            } else {
                let mut next = [
                    separation[0].div(&difference, budget)?,
                    separation[1].div(&difference, budget)?,
                ];
                if difference.is_negative() {
                    next.swap(0, 1);
                }
                support::intersect(&mut range, &next, budget)?;
            }
        }
    }
    Ok(range)
}

/// The complete admissible starting-phase fibre at a rate, including all correlations.
pub(super) fn phase(
    strips: &[Strip],
    rate: &Rational,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    let mut result = None;
    for strip in strips {
        budget.work(1, AT)?;
        let advance = rate.mul(&strip.time, budget)?;
        support::constrain(
            &mut result,
            [
                strip.phase[0].sub(&advance, budget)?,
                strip.phase[1].sub(&advance, budget)?,
            ],
            budget,
        )?;
    }
    result.ok_or(LcRelationError::Observation)
}

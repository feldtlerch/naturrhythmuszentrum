// SPDX-License-Identifier: AGPL-3.0-only
//! Exact half-open floor cells retain their complete contiguous integer range.

use super::{AT, PiAffineEndpoint, PiIntervalArc, invalid, rational_bytes, retained};
use crate::limits::Budget;
use crate::{Fault, InvalidCode, Rational, pi_bounds::CheckedPiBounds};
use std::cmp::Ordering;

/// Prove both endpoint cells; continuity then covers exactly every integer between them.
pub(super) fn cover(
    lifted: &[Rational; 2],
    cycles: &[Rational; 2],
    pi: &CheckedPiBounds,
    budget: &mut Budget,
) -> Result<(), Fault> {
    budget.work(1, AT)?;
    for value in lifted.iter().chain(cycles) {
        value.check(budget, AT)?;
    }
    if lifted[0].compare(&lifted[1], budget)? == Ordering::Greater {
        return Err(invalid(InvalidCode::DecisionBox));
    }
    if cycles[0].compare(&cycles[1], budget)? == Ordering::Greater {
        return Err(invalid(InvalidCode::DecisionBoundaryIntersection));
    }
    for (value, index) in lifted.iter().zip(cycles) {
        index.check(budget, AT)?;
        if !index.is_integer() {
            return Err(invalid(InvalidCode::DecisionIntegerRange));
        }
        let lower = turn(index, pi, budget)?;
        let upper = turn(&index.add(&Rational::one(), budget)?, pi, budget)?;
        let above_lower = value.compare(&lower[1], budget)? != Ordering::Less;
        let below_upper = value.compare(&upper[0], budget)? == Ordering::Less;
        if !above_lower || !below_upper {
            return Err(invalid(InvalidCode::DecisionBoundaryIntersection));
        }
    }
    Ok(())
}

/// Signed integer multiplication reverses the retained pi enclosure when necessary.
fn turn(
    index: &Rational,
    pi: &CheckedPiBounds,
    budget: &mut Budget,
) -> Result<[Rational; 2], Fault> {
    let coefficient = index.mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let lower = coefficient.mul(&pi.contract().lower, budget)?;
    let upper = coefficient.mul(&pi.contract().upper, budget)?;
    Ok(if coefficient.is_negative() {
        [upper, lower]
    } else {
        [lower, upper]
    })
}

/// Materialization is an exact range member, never a new bound on the original family.
pub(super) fn arc(
    lifted: &[Rational; 2],
    cycles: &[Rational; 2],
    index: Rational,
    budget: &mut Budget,
) -> Result<PiIntervalArc, Fault> {
    budget.work(1, AT)?;
    index.check(budget, AT)?;
    if !index.is_integer() {
        return Err(invalid(InvalidCode::DecisionIntegerRange));
    }
    let first = index.compare(&cycles[0], budget)?;
    let last = index.compare(&cycles[1], budget)?;
    if first == Ordering::Less || last == Ordering::Greater {
        return Err(invalid(InvalidCode::DecisionBoundaryIntersection));
    }
    let coefficient = index.mul(&Rational::from_ratio(-2, 1, budget)?, budget)?;
    let lower = if first == Ordering::Equal {
        PiAffineEndpoint {
            rational: lifted[0].clone(),
            pi: coefficient.clone(),
        }
    } else {
        PiAffineEndpoint {
            rational: Rational::zero(),
            pi: Rational::zero(),
        }
    };
    let upper_included = last == Ordering::Equal;
    let upper = if upper_included {
        PiAffineEndpoint {
            rational: lifted[1].clone(),
            pi: coefficient,
        }
    } else {
        PiAffineEndpoint {
            rational: Rational::zero(),
            pi: Rational::from_ratio(2, 1, budget)?,
        }
    };
    let mut bytes = rational_bytes(&index).saturating_add(512);
    for value in [&lower.rational, &lower.pi, &upper.rational, &upper.pi] {
        bytes = bytes.saturating_add(rational_bytes(value));
    }
    retained(bytes, budget)?;
    Ok(PiIntervalArc {
        index,
        lower,
        upper,
        upper_included,
    })
}

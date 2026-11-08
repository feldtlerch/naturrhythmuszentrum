// SPDX-License-Identifier: AGPL-3.0-only
//! Exact arithmetic primitives, not a producer or checker dispatch.

use super::{AT, Coefficient, invalid};
use crate::budget::{Budget, BudgetKind};
use crate::{Fault, InvalidCode, Rational};
use std::cmp::Ordering;

/// Exact powers are bounded by the caller before executing any shift.
pub(super) fn two_power(exponent: i64, budget: &mut Budget) -> Result<Rational, Fault> {
    Rational::from_ratio(2, 1, budget)?.pow(exponent, budget)
}

/// Copy or negate an exact magnitude through metered operations.
pub(super) fn abs(value: &Rational, budget: &mut Budget) -> Result<Rational, Fault> {
    if value.is_negative() {
        value.negated(budget)
    } else {
        value.add(&Rational::zero(), budget)
    }
}

/// A nonnegative upper rounding on a fixed proof grid prevents rational-size growth.
pub(super) fn upper(value: &Rational, budget: &mut Budget) -> Result<Rational, Fault> {
    let scale = two_power(128, budget)?;
    let scaled = value.mul(&scale, budget)?;
    let mut floor = scaled.floor(budget)?;
    if floor.compare(&scaled, budget)? != Ordering::Equal {
        floor = floor.add(&Rational::one(), budget)?;
    }
    floor.div(&scale, budget)
}

/// Producer quantization uses an exact binade search and integral ties-to-even.
pub(super) fn coefficient(
    value: &Rational,
    bits: u32,
    budget: &mut Budget,
) -> Result<Coefficient, Fault> {
    let (n, d) = value.component_bits();
    let mut binade = i64::try_from(n)
        .map_err(|_| Fault::capacity(BudgetKind::ComponentBits, AT))?
        - i64::try_from(d).map_err(|_| Fault::capacity(BudgetKind::ComponentBits, AT))?;
    if value.compare(&two_power(binade, budget)?, budget)? == Ordering::Less {
        binade -= 1;
    }
    let mut exponent = binade - i64::from(bits - 1);
    let scaled = value.div(&two_power(exponent, budget)?, budget)?;
    let floor = scaled.floor(budget)?;
    let (integer, denominator) = floor
        .small_ratio(budget)?
        .ok_or_else(|| invalid(InvalidCode::Expm1Coefficient))?;
    if denominator != 1 {
        return Err(invalid(InvalidCode::Expm1Coefficient));
    }
    let mut significand =
        u64::try_from(integer).map_err(|_| invalid(InvalidCode::Expm1Coefficient))?;
    let remainder = scaled.sub(&floor, budget)?;
    let order = remainder.compare(&Rational::from_ratio(1, 2, budget)?, budget)?;
    if order == Ordering::Greater || (order == Ordering::Equal && significand % 2 != 0) {
        significand += 1;
    }
    if significand == (1_u64 << bits) {
        significand /= 2;
        exponent += 1;
    }
    Ok(Coefficient {
        significand,
        exponent: i32::try_from(exponent).map_err(|_| invalid(InvalidCode::Expm1Coefficient))?,
    })
}

/// Reserve all fixed record slots before vectors are allocated.
pub(super) fn capacity(degree: u32, budget: &Budget) -> Result<(), Fault> {
    budget.check(BudgetKind::PolynomialDegree, u64::from(degree), AT)?;
    budget.check(
        BudgetKind::ProofNodes,
        u64::from(degree).saturating_mul(3).saturating_add(8),
        AT,
    )?;
    budget.check(
        BudgetKind::StagedPayloadBytes,
        u64::from(degree).saturating_mul(4096).saturating_add(8192),
        AT,
    )
}

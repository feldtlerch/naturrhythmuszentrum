// SPDX-License-Identifier: AGPL-3.0-only
//! Exact rate/product inclusion and same-coordinate horizon order accompany the original measurement implication.

use super::{AT, Cover, LcClockRefinementSuccess, LcRelationError, inclusion, storage};
use crate::{Budget, LcHorizon, LcHorizonValue};
use nrz_cas::Rational;
use std::cmp::Ordering;

/// Original whole-question inclusion is audited by exact projections and a common-reference horizon comparison.
pub(super) fn success(
    previous: &Cover,
    old_horizon: &LcHorizon,
    cover: Cover,
    budget: &mut Budget,
) -> Result<LcClockRefinementSuccess, LcRelationError> {
    inclusion::contained(previous.rate(), cover.rate(), budget)?;
    let products = [
        product(previous.rate(), budget)?,
        product(cover.rate(), budget)?,
    ];
    inclusion::contained(&products[0], &products[1], budget)?;
    let horizon = cover.horizon(
        old_horizon.coordinate(),
        old_horizon.mathematical().reference(),
        budget,
    )?;
    let margin = order(old_horizon, &horizon, budget)?;
    let bytes = products
        .iter()
        .flatten()
        .chain(margin.iter())
        .fold(2_048_u64, |sum, value| {
            sum.saturating_add(storage::rational(value))
        })
        .saturating_add(cover.bytes())
        .saturating_add(horizon.retained_payload_bytes());
    storage::retained(bytes, budget)?;
    Ok(LcClockRefinementSuccess {
        cover,
        horizon,
        products,
        margin,
    })
}

/// Positive inversion preserves the exact LC-product direction and reverses endpoint order.
fn product(rate: &[Rational; 2], budget: &mut Budget) -> Result<[Rational; 2], LcRelationError> {
    budget.work(1, AT)?;
    Ok([
        Rational::one().div(&rate[1].pow(2, budget)?, budget)?,
        Rational::one().div(&rate[0].pow(2, budget)?, budget)?,
    ])
}

/// The same future coordinate and reference are mandatory before comparing exact affine-pi thresholds.
fn order(
    previous: &LcHorizon,
    refined: &LcHorizon,
    budget: &mut Budget,
) -> Result<Option<Rational>, LcRelationError> {
    budget.work(1, AT)?;
    let coordinate = previous.coordinate() == refined.coordinate();
    let reference = previous
        .mathematical()
        .reference()
        .compare(refined.mathematical().reference(), budget)?
        == Ordering::Equal;
    if !coordinate || !reference {
        return Err(LcRelationError::Containment);
    }
    match (previous.value(), refined.value()) {
        (LcHorizonValue::Finite(old), LcHorizonValue::Finite(new)) => {
            let (old_offset, old_pi) = old.coefficients();
            let (new_offset, new_pi) = new.coefficients();
            let offset = new_offset.sub(old_offset, budget)?;
            let coefficient = new_pi.sub(old_pi, budget)?;
            let pi = previous.mathematical().pi_bounds().contract();
            let bound = if coefficient.is_negative() {
                &pi.upper
            } else {
                &pi.lower
            };
            let margin = offset.add(&coefficient.mul(bound, budget)?, budget)?;
            if margin.is_negative() {
                return Err(LcRelationError::UnprovedOrder);
            }
            Ok(Some(margin))
        }
        (LcHorizonValue::Finite(_) | LcHorizonValue::Infinite, LcHorizonValue::Infinite) => {
            Ok(None)
        }
        (LcHorizonValue::Infinite | LcHorizonValue::Undecided, _)
        | (_, LcHorizonValue::Undecided) => Err(LcRelationError::UnprovedOrder),
    }
}

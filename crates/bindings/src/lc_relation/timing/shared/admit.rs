// SPDX-License-Identifier: AGPL-3.0-only
//! Every original time restriction must follow from the declared two-layer support.

use super::{AT, Bounds, LcRelation, LcRelationError, Rational, storage};
use crate::{Budget, LcInterval, LcQuantity as Q};
use std::cmp::Ordering;

/// Original rate and optional physical phase intersections, kept beside the clock domain.
#[derive(Debug)]
pub(super) struct Premises {
    /// Positive finite rate support.
    pub(super) rate: Bounds,
    /// Original finite physical phase support; absent for the complete common gauge.
    pub(super) initial: Option<Bounds>,
}

/// No timestamp or physical phase restriction may disappear from the auxiliary coordinates.
pub(super) fn question(
    relation: &LcRelation,
    nominal: &[Rational],
    errors: &[Bounds],
    offset: &Bounds,
    budget: &mut Budget,
) -> Result<Premises, LcRelationError> {
    budget.work(1, AT)?;
    let same_count = nominal.len() == relation.observations();
    let all_errors = errors.len() == nominal.len();
    if !same_count || !all_errors || nominal.len() < 2 {
        return Err(LcRelationError::Observation);
    }
    storage::retained(storage::size(nominal.len())?.saturating_mul(8_192), budget)?;
    for time in nominal {
        budget.work(1, AT)?;
        time.check(budget, AT)?;
    }
    let mut rate = None;
    let mut phase = None;
    for input in relation.inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            Q::Rate | Q::AngularFrequency => constrain(
                &mut rate,
                super::super::admit::bounds(input.interval(), budget)?,
                budget,
            )?,
            Q::InitialPhase => constrain(
                &mut phase,
                super::super::admit::bounds(input.interval(), budget)?,
                budget,
            )?,
            Q::Time(index) => {
                let image = [
                    nominal[index]
                        .add(&errors[index][0], budget)?
                        .add(&offset[0], budget)?,
                    nominal[index]
                        .add(&errors[index][1], budget)?
                        .add(&offset[1], budget)?,
                ];
                contains_image(input.interval(), &image, budget)?;
            }
            _ => (),
        }
    }
    let rate = rate.ok_or(LcRelationError::Projection)?;
    if rate[0].is_zero() || rate[0].is_negative() {
        return Err(LcRelationError::Positivity);
    }
    Ok(Premises {
        rate,
        initial: phase,
    })
}

/// Repeated original bounds intersect; later declarations never overwrite a restriction.
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

/// Full-image containment retains original total time bounds without independent replacement.
fn contains_image(
    interval: &LcInterval,
    image: &Bounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    for (endpoint, image, forbidden) in [
        (interval.lower(), &image[0], Ordering::Greater),
        (interval.upper(), &image[1], Ordering::Less),
    ] {
        if let Some(endpoint) = endpoint {
            let endpoint = endpoint.as_rational().ok_or(LcRelationError::Projection)?;
            if endpoint.compare(image, budget)? == forbidden {
                return Err(LcRelationError::Containment);
            }
        }
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Original total timestamps intersect one common offset without replacing independent error supports.

use super::{AT, Bounds, LcRelation, LcRelationError, Rational, storage};
use crate::lc_relation::calibration;
use crate::{Budget, LcQuantity as Q};
use std::cmp::Ordering;

/// Every original physical and measurement restriction remains beside its exact intersections.
#[derive(Debug)]
pub(super) struct Premises {
    /// Positive original finite rate support.
    pub(super) rate: Bounds,
    /// Optional repeated physical initial-phase intersection.
    pub(super) initial: Option<Bounds>,
    /// Total timestamps after every original restriction and the declared measurement image.
    pub(super) times: Vec<Bounds>,
    /// One surviving common offset, distinct from its original declaration.
    pub(super) support: Bounds,
    /// Whether original physical timestamps require the restricted joint projection.
    pub(super) narrow: bool,
}

/// Every repeated physical timestamp contributes before the common offset is projected.
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
    let images = images(nominal, errors, offset, budget)?;
    let mut times = images.clone();
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
                calibration::clip_interval(&mut times[index], input.interval(), budget)?;
            }
            _ => (),
        }
    }
    let rate = rate.ok_or(LcRelationError::Projection)?;
    if rate[0].is_zero() || rate[0].is_negative() {
        return Err(LcRelationError::Positivity);
    }
    let (support, narrow) = supports(nominal, errors, offset, &times, &images, budget)?;
    Ok(Premises {
        rate,
        initial: phase,
        times,
        support,
        narrow,
    })
}

/// Every original total time contributes to the identical shared offset before projection.
fn supports(
    nominal: &[Rational],
    errors: &[Bounds],
    offset: &Bounds,
    times: &[Bounds],
    images: &[Bounds],
    budget: &mut Budget,
) -> Result<(Bounds, bool), LcRelationError> {
    let mut support = offset.clone();
    let mut narrow = false;
    for (index, time) in times.iter().enumerate() {
        budget.work(1, AT)?;
        let lower_same = time[0].compare(&images[index][0], budget)? == Ordering::Equal;
        let upper_same = time[1].compare(&images[index][1], budget)? == Ordering::Equal;
        narrow |= !lower_same || !upper_same;
        let image = [
            time[0]
                .sub(&nominal[index], budget)?
                .sub(&errors[index][1], budget)?,
            time[1]
                .sub(&nominal[index], budget)?
                .sub(&errors[index][0], budget)?,
        ];
        calibration::intersect(&mut support, &image, budget)?;
    }
    Ok((support, narrow))
}

/// Original independent errors and the declared common offset give finite physical time images.
fn images(
    nominal: &[Rational],
    errors: &[Bounds],
    offset: &Bounds,
    budget: &mut Budget,
) -> Result<Vec<Bounds>, LcRelationError> {
    nominal
        .iter()
        .zip(errors)
        .map(|(time, error)| {
            budget.work(1, AT)?;
            time.check(budget, AT)?;
            Ok([
                time.add(&error[0], budget)?.add(&offset[0], budget)?,
                time.add(&error[1], budget)?.add(&offset[1], budget)?,
            ])
        })
        .collect()
}

/// Repeated bounds retain every original declaration instead of overwriting it.
fn constrain(
    slot: &mut Option<Bounds>,
    next: Bounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    if let Some(previous) = slot {
        calibration::intersect(previous, &next, budget)?;
    } else {
        *slot = Some(next);
    }
    Ok(())
}

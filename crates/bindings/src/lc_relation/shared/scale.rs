// SPDX-License-Identifier: AGPL-3.0-only
//! Positive scale eliminations are monotone maps of one retained measurement domain.

use super::super::amplitude::scale::coordinate;
use super::{
    AT, Bounds, LcInterval, LcRelationError, LcSharedMeasurementCover, LcSharedMeasurements,
    Rational,
};
use crate::{Budget, LcQuantity as Q};
use std::cmp::Ordering;

/// A measured amplitude alone leaves the independent element scale unbounded.
pub(super) fn bounded(
    amplitude: &LcSharedMeasurements,
    quantity: Q,
) -> Result<bool, LcRelationError> {
    match quantity {
        Q::Rate | Q::AngularFrequency | Q::Product | Q::NaturalTime => Ok(true),
        Q::VoltageAmplitude
        | Q::CurrentAmplitude
        | Q::Inductance
        | Q::Capacitance
        | Q::AmplitudeRatio
        | Q::Energy => Ok(quantity == amplitude.channel() || amplitude.energy().is_some()),
        _ => Err(LcRelationError::Projection),
    }
}

/// Physical projections are positive monomials, so their rational extrema occur at corners.
pub(super) fn bounds(
    cover: &LcSharedMeasurementCover,
    quantity: Q,
    budget: &mut Budget,
) -> Result<LcInterval, LcRelationError> {
    budget.work(1, AT)?;
    if matches!(
        quantity,
        Q::Rate | Q::AngularFrequency | Q::Product | Q::NaturalTime
    ) {
        return cover.0.effective.bounds(quantity, budget);
    }
    if !bounded(cover.measurements(), quantity)? {
        return LcInterval::new(Some(Rational::zero().into()), None, budget);
    }
    let magnitude = cover.measurements().physical_amplitude();
    if quantity == cover.measurements().channel() {
        return interval(magnitude.clone(), budget);
    }
    let energy = cover
        .measurements()
        .energy()
        .ok_or(LcRelationError::Projection)?;
    let mut range: Option<Bounds> = None;
    for rate in cover.rate() {
        for amplitude in magnitude {
            for energy in energy {
                budget.work(1, AT)?;
                let value = coordinate(
                    cover.measurements().channel(),
                    quantity,
                    rate,
                    amplitude,
                    energy,
                    budget,
                )?;
                if let Some(bounds) = &mut range {
                    if value.compare(&bounds[0], budget)? == Ordering::Less {
                        bounds[0].clone_from(&value);
                    }
                    if value.compare(&bounds[1], budget)? == Ordering::Greater {
                        bounds[1].clone_from(&value);
                    }
                } else {
                    range = Some([value.clone(), value]);
                }
            }
        }
    }
    interval(range.ok_or(LcRelationError::Projection)?, budget)
}

/// Exact coherent endpoints construct the ordinary typed interval output.
fn interval([lower, upper]: Bounds, budget: &mut Budget) -> Result<LcInterval, LcRelationError> {
    LcInterval::new(Some(lower.into()), Some(upper.into()), budget)
}

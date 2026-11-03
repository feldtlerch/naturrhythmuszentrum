// SPDX-License-Identifier: AGPL-3.0-only
//! Positive scale eliminations are monotone maps of one retained measurement domain.

use super::{
    AT, Bounds, LcAmplitudeCover, LcAmplitudeSamples, LcInterval, LcRelationError, Rational,
};
use crate::{Budget, LcQuantity as Q};
use std::cmp::Ordering;

/// A measured amplitude alone leaves the independent element scale unbounded.
pub(super) fn bounded(
    amplitude: &LcAmplitudeSamples,
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
    cover: &LcAmplitudeCover,
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
    if !bounded(cover.amplitude(), quantity)? {
        return LcInterval::new(Some(Rational::zero().into()), None, budget);
    }
    let magnitude = cover.amplitude().physical_amplitude();
    if quantity == cover.amplitude().channel() {
        return interval(magnitude.clone(), budget);
    }
    let energy = cover
        .amplitude()
        .energy()
        .ok_or(LcRelationError::Projection)?;
    let mut range: Option<Bounds> = None;
    for rate in cover.rate() {
        for amplitude in magnitude {
            for energy in energy {
                budget.work(1, AT)?;
                let value = coordinate(
                    cover.amplitude().channel(),
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

/// Original physical equations solve every scale from positive rate, amplitude and energy.
pub(in crate::lc_relation) fn coordinate(
    channel: Q,
    quantity: Q,
    rate: &Rational,
    amplitude: &Rational,
    energy: &Rational,
    budget: &mut Budget,
) -> Result<Rational, LcRelationError> {
    budget.work(1, AT)?;
    if quantity == channel {
        return Ok(amplitude.clone());
    }
    if quantity == Q::Energy {
        return Ok(energy.clone());
    }
    let twice = energy.mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let square = amplitude.pow(2, budget)?;
    let measured_voltage = channel == Q::VoltageAmplitude;
    match quantity {
        Q::VoltageAmplitude | Q::CurrentAmplitude => {
            Ok(twice.mul(rate, budget)?.div(amplitude, budget)?)
        }
        Q::Inductance | Q::Capacitance => {
            let direct = (quantity == Q::Capacitance) == measured_voltage;
            if direct {
                Ok(twice.div(&square, budget)?)
            } else {
                Ok(square.div(&twice.mul(&rate.pow(2, budget)?, budget)?, budget)?)
            }
        }
        Q::AmplitudeRatio => {
            let product = twice.mul(rate, budget)?;
            if measured_voltage {
                Ok(square.div(&product, budget)?)
            } else {
                Ok(product.div(&square, budget)?)
            }
        }
        _ => Err(LcRelationError::Projection),
    }
}

/// Exact coherent endpoints construct the ordinary typed interval output.
fn interval([lower, upper]: Bounds, budget: &mut Budget) -> Result<LcInterval, LcRelationError> {
    LcInterval::new(Some(lower.into()), Some(upper.into()), budget)
}

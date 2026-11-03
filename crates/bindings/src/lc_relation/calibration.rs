// SPDX-License-Identifier: AGPL-3.0-only
//! Repeated positive readings and their single calibration retain an exact conditional fibre.

use super::{AT, LcInterval, LcRelationError, storage};
use crate::{Budget, ExactConstant};
use nrz_cas::Rational;
use std::cmp::Ordering;

/// Finite positive rational endpoints in coherent coordinates.
pub(super) type Bounds = [Rational; 2];

/// Measurement data only; physical owners retain this beside their complete original question.
#[derive(Debug)]
pub(super) struct Calibration {
    /// Every independent original reading support.
    pub(super) samples: Vec<LcInterval>,
    /// Common reading support implied by one physical value and one calibration.
    pub(super) measured: Bounds,
    /// Single positive multiplicative calibration shared by all readings.
    pub(super) gain: Bounds,
    /// Exact physical-value marginal before any additional original restrictions.
    pub(super) image: Bounds,
    /// Conservative new retained metadata.
    pub(super) bytes: u64,
}

impl Calibration {
    /// Positive product elimination preserves all original samples and the shared gain.
    pub(super) fn new(
        samples: Vec<LcInterval>,
        gain: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        storage::retained(storage::size(samples.len())?.saturating_mul(2_048), budget)?;
        let gain = positive_bounds(gain, budget)?;
        let mut measured =
            positive_bounds(samples.first().ok_or(LcRelationError::Observation)?, budget)?;
        for sample in &samples {
            intersect(&mut measured, &positive_bounds(sample, budget)?, budget)?;
        }
        let image = [
            measured[0].mul(&gain[0], budget)?,
            measured[1].mul(&gain[1], budget)?,
        ];
        let bytes = gain
            .iter()
            .chain(&measured)
            .chain(&image)
            .fold(1_024_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            });
        let bytes = samples
            .iter()
            .fold(bytes, |sum, sample| sum.saturating_add(sample.bytes()));
        storage::retained(bytes, budget)?;
        Ok(Self {
            samples,
            measured,
            gain,
            image,
            bytes,
        })
    }

    /// The gain fibre at a physical value preserves the measurement correlation exactly.
    pub(super) fn gain_at_value(
        &self,
        value: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        positive(value, budget)?;
        let mut fibre = [
            value.div(&self.measured[1], budget)?,
            value.div(&self.measured[0], budget)?,
        ];
        intersect(&mut fibre, &self.gain, budget)?;
        Ok(fibre)
    }

    /// Every reading is checked with the identical calibration, never separate marginal gains.
    pub(super) fn check(
        &self,
        value: &ExactConstant,
        gain: &Rational,
        budget: &mut Budget,
    ) -> Result<(), LcRelationError> {
        contains(&self.gain, gain, budget)?;
        let measured = value.div(&gain.clone().into(), budget)?;
        for sample in &self.samples {
            budget.work(1, AT)?;
            if !sample.contains(&measured, budget)? {
                return Err(LcRelationError::Containment);
            }
        }
        Ok(())
    }
}

/// Strict positivity and rational size checks spend the caller's Work.
pub(super) fn positive(value: &Rational, budget: &mut Budget) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    value.check(budget, AT)?;
    if value.is_negative() || value.is_zero() {
        return Err(LcRelationError::Positivity);
    }
    Ok(())
}

/// The closed measurement adapter admits finite strictly positive rational intervals.
pub(super) fn positive_bounds(
    interval: &LcInterval,
    budget: &mut Budget,
) -> Result<Bounds, LcRelationError> {
    budget.work(1, AT)?;
    let lower = interval
        .lower()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    let upper = interval
        .upper()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    positive(lower, budget)?;
    positive(upper, budget)?;
    Ok([lower.clone(), upper.clone()])
}

/// An exact intersection may refuse without asserting a global impossibility certificate.
pub(super) fn intersect(
    left: &mut Bounds,
    right: &Bounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
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

/// Membership retains both positive-domain and endpoint evidence.
pub(super) fn contains(
    range: &Bounds,
    value: &Rational,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    positive(value, budget)?;
    let above = value.compare(&range[0], budget)? != Ordering::Less;
    let below = value.compare(&range[1], budget)? != Ordering::Greater;
    if !above || !below {
        return Err(LcRelationError::Containment);
    }
    Ok(())
}

/// Construct an ordinary coherent interval from exact rational endpoints.
pub(super) fn interval(range: &Bounds, budget: &mut Budget) -> Result<LcInterval, LcRelationError> {
    LcInterval::new(
        Some(range[0].clone().into()),
        Some(range[1].clone().into()),
        budget,
    )
}

/// Preserve broad exact restrictions; a narrowing endpoint must fit the rational projection fragment.
pub(super) fn clip_interval(
    image: &mut Bounds,
    interval: &LcInterval,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    let mut next = image.clone();
    if let Some(lower) = interval.lower()
        && !super::ordered(lower, &image[0].clone().into(), budget)?
    {
        next[0] = lower
            .as_rational()
            .ok_or(LcRelationError::Projection)?
            .clone();
    }
    if let Some(upper) = interval.upper()
        && !super::ordered(&image[1].clone().into(), upper, budget)?
    {
        next[1] = upper
            .as_rational()
            .ok_or(LcRelationError::Projection)?
            .clone();
    }
    intersect(image, &next, budget)
}

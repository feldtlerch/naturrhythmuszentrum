// SPDX-License-Identifier: AGPL-3.0-only
//! Admission preserves repeated measurements and refuses omitted physical restrictions.

use super::{
    AT, Bounds, LcAmplitudeSamples, LcInterval, LcRelationError, Question, Rational, storage,
};
use crate::{Budget, ExactConstant, LcQuantity as Q, LcSampleTimes};
use std::{cmp::Ordering, rc::Rc};

/// Every input domain is finite and strictly positive for this oscillating amplitude adapter.
pub(super) fn bounds(
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

/// Strict positivity is checked with the caller's cumulative arithmetic resources.
pub(super) fn positive(value: &Rational, budget: &mut Budget) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    value.check(budget, AT)?;
    if value.is_negative() || value.is_zero() {
        return Err(LcRelationError::Positivity);
    }
    Ok(())
}

/// Intersections preserve all repeated original declarations.
pub(super) fn intersect(
    left: &mut Bounds,
    right: &Bounds,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
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

/// Membership is checked before any physical reconstruction.
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

/// One gain and one physical amplitude require a common measured value in every sample box.
pub(super) fn question(
    timing: LcSampleTimes,
    channel: Q,
    samples: Vec<LcInterval>,
    gain: &LcInterval,
    budget: &mut Budget,
) -> Result<LcAmplitudeSamples, LcRelationError> {
    budget.work(1, AT)?;
    if !matches!(channel, Q::VoltageAmplitude | Q::CurrentAmplitude) {
        return Err(LcRelationError::Projection);
    }
    storage::retained(storage::size(samples.len())?.saturating_mul(2_048), budget)?;
    let gain = bounds(gain, budget)?;
    let first = samples.first().ok_or(LcRelationError::Observation)?;
    let mut measured = bounds(first, budget)?;
    for sample in &samples {
        let next = bounds(sample, budget)?;
        intersect(&mut measured, &next, budget)?;
    }
    let image = [
        measured[0].mul(&gain[0], budget)?,
        measured[1].mul(&gain[1], budget)?,
    ];
    let energy = original(&timing, channel, &image, budget)?;
    let bytes = gain
        .iter()
        .chain(&measured)
        .chain(energy.iter().flatten())
        .fold(2_048_u64, |sum, value| {
            sum.saturating_add(storage::rational(value))
        });
    let bytes = samples
        .iter()
        .fold(bytes, |sum, interval| sum.saturating_add(interval.bytes()));
    storage::retained(
        bytes
            .saturating_add(timing.retained_payload_bytes())
            .saturating_add(timing.relation().retained_payload_bytes()),
        budget,
    )?;
    Ok(LcAmplitudeSamples(Rc::new(Question {
        timing,
        channel,
        samples,
        measured,
        gain,
        energy,
        bytes,
    })))
}

/// Every original scale restriction is retained by exact elimination or explicitly refused.
fn original(
    timing: &LcSampleTimes,
    channel: Q,
    image: &Bounds,
    budget: &mut Budget,
) -> Result<Option<Bounds>, LcRelationError> {
    let mut energy = None;
    for input in timing.relation().inputs() {
        budget.work(1, AT)?;
        if input.quantity() == channel {
            for endpoint in image {
                if !input
                    .interval()
                    .contains(&endpoint.clone().into(), budget)?
                {
                    return Err(LcRelationError::Containment);
                }
            }
        } else {
            match input.quantity() {
                Q::Energy => {
                    let next = bounds(input.interval(), budget)?;
                    if let Some(previous) = &mut energy {
                        intersect(previous, &next, budget)?;
                    } else {
                        energy = Some(next);
                    }
                }
                Q::Time(_) | Q::Phase(_) | Q::InitialPhase | Q::Rate | Q::AngularFrequency => (),
                _ => return Err(LcRelationError::Projection),
            }
        }
    }
    Ok(energy)
}

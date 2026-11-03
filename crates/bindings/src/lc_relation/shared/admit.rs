// SPDX-License-Identifier: AGPL-3.0-only
//! Admission preserves repeated measurements and refuses omitted physical restrictions.

use super::{AT, Bounds, LcInterval, LcRelationError, LcSharedMeasurements, Question, storage};
use crate::{Budget, LcQuantity as Q, LcSharedClockPhase};
use std::rc::Rc;

pub(super) use super::super::calibration::{
    contains, intersect, positive, positive_bounds as bounds,
};

/// One gain and one physical amplitude require a common measured value in every sample box.
pub(super) fn question(
    phase: LcSharedClockPhase,
    channel: Q,
    samples: Vec<LcInterval>,
    gain: &LcInterval,
    budget: &mut Budget,
) -> Result<LcSharedMeasurements, LcRelationError> {
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
    let energy = original(&phase, channel, &image, budget)?;
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
            .saturating_add(phase.retained_payload_bytes())
            .saturating_add(phase.clock().retained_payload_bytes())
            .saturating_add(phase.relation().retained_payload_bytes()),
        budget,
    )?;
    Ok(LcSharedMeasurements(Rc::new(Question {
        phase,
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
    phase: &LcSharedClockPhase,
    channel: Q,
    image: &Bounds,
    budget: &mut Budget,
) -> Result<Option<Bounds>, LcRelationError> {
    let mut energy = None;
    for input in phase.relation().inputs() {
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

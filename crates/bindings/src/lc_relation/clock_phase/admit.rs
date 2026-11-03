// SPDX-License-Identifier: AGPL-3.0-only
//! Original physical strips stay retained even when they narrow a shared-offset measurement image.

use super::{AT, Bounds, LcInterval, LcRelation, LcRelationError};
use crate::{Budget, ExactConstant, LcQuantity as Q};

pub(super) use crate::lc_relation::phase::admit::{bounds, contains, intersect};

/// Preserve the established full-image path and explicitly select the joint narrower-strip path.
pub(super) fn question(
    relation: &LcRelation,
    samples: &[LcInterval],
    offset: &Bounds,
    budget: &mut Budget,
) -> Result<(Option<Bounds>, bool), LcRelationError> {
    budget.work(1, AT)?;
    if samples.len() != relation.observations() {
        return Err(LcRelationError::Observation);
    }
    for sample in samples {
        budget.work(1, AT)?;
        if sample.lower().is_none() || sample.upper().is_none() {
            return Err(LcRelationError::Projection);
        }
    }
    let exact_offset = offset.clone().map(ExactConstant::from);
    let mut initial = None;
    let mut narrow = false;
    for input in relation.inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            Q::InitialPhase => {
                let next = bounds(input.interval(), budget)?;
                if let Some(previous) = &mut initial {
                    intersect(previous, &next, budget)?;
                } else {
                    initial = Some(next);
                }
            }
            Q::Phase(index) => {
                let sample = &samples[index];
                let lower = sample
                    .lower()
                    .ok_or(LcRelationError::Projection)?
                    .add(&exact_offset[0], budget)?;
                let upper = sample
                    .upper()
                    .ok_or(LcRelationError::Projection)?
                    .add(&exact_offset[1], budget)?;
                let lower_inside = input.interval().contains(&lower, budget)?;
                let upper_inside = input.interval().contains(&upper, budget)?;
                narrow |= !lower_inside || !upper_inside;
            }
            _ => (),
        }
    }
    Ok((initial, narrow))
}

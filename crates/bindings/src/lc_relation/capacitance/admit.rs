// SPDX-License-Identifier: AGPL-3.0-only
//! Eliminate only an explicitly retained independent positive capacitor interval.

use super::{AT, LcQuantity, LcRelation, LcRelationError, Rational, storage};
use crate::{Budget, ExactConstant};
use std::cmp::Ordering;

/// Every other original physical scale refuses, making the remaining rate projection exact.
pub(super) fn inputs(
    relation: &LcRelation,
    budget: &mut Budget,
) -> Result<(LcRelation, [Rational; 2]), LcRelationError> {
    budget.work(1, AT)?;
    storage::retained(
        relation
            .retained_payload_bytes()
            .saturating_mul(2)
            .saturating_add(2_048),
        budget,
    )?;
    let mut capacitance: Option<[Rational; 2]> = None;
    let mut phases = Vec::with_capacity(relation.inputs().len());
    for input in relation.inputs() {
        budget.work(1, AT)?;
        match input.quantity() {
            LcQuantity::Capacitance => {
                let lower = input
                    .interval()
                    .lower()
                    .and_then(ExactConstant::as_rational)
                    .ok_or(LcRelationError::Projection)?;
                let upper = input
                    .interval()
                    .upper()
                    .and_then(ExactConstant::as_rational)
                    .ok_or(LcRelationError::Projection)?;
                lower.check(budget, AT)?;
                upper.check(budget, AT)?;
                let mut next = [lower.clone(), upper.clone()];
                if let Some(previous) = &capacitance {
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
                capacitance = Some(next);
            }
            LcQuantity::Rate
            | LcQuantity::AngularFrequency
            | LcQuantity::InitialPhase
            | LcQuantity::Phase(_)
            | LcQuantity::Time(_) => phases.push(input.clone()),
            _ => return Err(LcRelationError::Projection),
        }
    }
    let capacitance = capacitance.ok_or(LcRelationError::Projection)?;
    if capacitance[0].is_zero() || capacitance[0].is_negative() {
        return Err(LcRelationError::Positivity);
    }
    let phase_relation = LcRelation::new(
        relation.epoch().to_owned(),
        relation.observations(),
        phases,
        budget,
    )?;
    Ok((phase_relation, capacitance))
}

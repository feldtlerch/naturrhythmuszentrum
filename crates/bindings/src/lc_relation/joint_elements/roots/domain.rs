// SPDX-License-Identifier: AGPL-3.0-only
//! A complete original zero-epoch inventory leaves exactly the positive element rectangle and free amplitude.

use super::{AT, LcRelationError, calibration, storage};
use crate::lc_relation::phase::admit::bounds;
use crate::{Budget, LcQuantity as Q, LcRelation};
use nrz_cas::Rational;

/// Every original observation has known zero phase at the common epoch, fixing every cycle to zero.
#[derive(Debug)]
pub(super) struct Domain {
    /// Original positive inductance and capacitance intersections, in that order.
    pub(super) elements: [[Rational; 2]; 2],
    /// Complete exact product interval before its decreasing positive inverse square root.
    pub(super) product: [Rational; 2],
}

impl Domain {
    /// All original declarations participate; incompatible timing and additional scales refuse.
    pub(super) fn check(
        relation: &LcRelation,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        if relation.observations() == 0 {
            return Err(LcRelationError::Observation);
        }
        storage::retained(
            relation
                .retained_payload_bytes()
                .saturating_add(storage::size(relation.observations())?.saturating_mul(8_192)),
            budget,
        )?;
        budget.work(storage::size(relation.observations())?, AT)?;
        let mut observations = vec![[None, None]; relation.observations()];
        let mut elements = [None, None];
        let mut initial = None;
        for input in relation.inputs() {
            budget.work(1, AT)?;
            let slot = match input.quantity() {
                Q::Inductance => &mut elements[0],
                Q::Capacitance => &mut elements[1],
                Q::InitialPhase => &mut initial,
                Q::Time(index) => &mut observations[index][0],
                Q::Phase(index) => &mut observations[index][1],
                _ => return Err(LcRelationError::Projection),
            };
            let next = bounds(input.interval(), budget)?;
            if let Some(previous) = slot {
                calibration::intersect(previous, &next, budget)?;
            } else {
                *slot = Some(next);
            }
        }
        zero(initial.as_ref(), budget)?;
        for observation in &observations {
            for value in observation {
                zero(value.as_ref(), budget)?;
            }
        }
        let [inductance, capacitance] = elements;
        let elements = [
            inductance.ok_or(LcRelationError::Projection)?,
            capacitance.ok_or(LcRelationError::Projection)?,
        ];
        for element in &elements {
            calibration::positive(&element[0], budget)?;
        }
        let product = [
            elements[0][0].mul(&elements[1][0], budget)?,
            elements[0][1].mul(&elements[1][1], budget)?,
        ];
        Ok(Self { elements, product })
    }

    /// Every positive product in the rectangle image has a nonempty conditional capacitance interval.
    pub(super) fn capacitance(
        &self,
        product: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        calibration::contains(&self.product, product, budget)?;
        let mut fibre = [
            product.div(&self.elements[0][1], budget)?,
            product.div(&self.elements[0][0], budget)?,
        ];
        calibration::intersect(&mut fibre, &self.elements[1], budget)?;
        Ok(fibre)
    }

    /// Conservative original intersections and derived product endpoints.
    pub(super) fn bytes(&self) -> u64 {
        self.elements
            .iter()
            .flatten()
            .chain(&self.product)
            .fold(1_024_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            })
    }
}

/// Exact zero timestamps and phases fix all original integer lifts without any finite alias search.
fn zero(value: Option<&[Rational; 2]>, budget: &mut Budget) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    let value = value.ok_or(LcRelationError::Projection)?;
    if !value[0].is_zero() || !value[1].is_zero() {
        return Err(LcRelationError::Projection);
    }
    Ok(())
}

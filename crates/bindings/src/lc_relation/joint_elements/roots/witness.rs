// SPDX-License-Identifier: AGPL-3.0-only
//! Positive root reconstruction rechecks every original physical equation and zero-epoch observation.

use super::{AT, LcRelationError, calibration, storage};
use crate::{Budget, ExactConstant, LcObservationValue, LcRelation, LcValues, LcWitness};
use nrz_cas::Rational;

/// Product and conditional capacitance reconstruct the same original simultaneous physical tuple.
pub(super) fn restore(
    relation: &LcRelation,
    product: &Rational,
    capacitance: &Rational,
    voltage: &Rational,
    budget: &mut Budget,
) -> Result<LcWitness, LcRelationError> {
    budget.work(1, AT)?;
    calibration::positive(product, budget)?;
    calibration::positive(capacitance, budget)?;
    calibration::positive(voltage, budget)?;
    storage::retained(
        relation
            .retained_payload_bytes()
            .saturating_add(storage::size(relation.observations())?.saturating_mul(4_096)),
        budget,
    )?;
    let inductance = product.div(capacitance, budget)?;
    let root = ExactConstant::from(product.clone())
        .positive()
        .ok_or(LcRelationError::Positivity)?
        .sqrt(budget)?;
    let rate = ExactConstant::one().div(&root, budget)?;
    let current = rate.mul(&capacitance.mul(voltage, budget)?.into(), budget)?;
    let energy = capacitance
        .mul(&voltage.pow(2, budget)?, budget)?
        .div(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let mut observations = Vec::with_capacity(relation.observations());
    for _ in 0..relation.observations() {
        budget.work(1, AT)?;
        observations.push(LcObservationValue {
            time: ExactConstant::zero(),
            phase: Some(ExactConstant::zero()),
            cycle: Some(Rational::zero()),
        });
    }
    relation.check_witness(
        LcValues {
            inductance: inductance.into(),
            capacitance: capacitance.clone().into(),
            rate,
            initial_phase: Some(ExactConstant::zero()),
            voltage: voltage.clone().into(),
            current,
            energy: energy.into(),
            observations,
        },
        budget,
    )
}

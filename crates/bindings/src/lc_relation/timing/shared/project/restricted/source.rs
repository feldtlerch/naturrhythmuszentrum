// SPDX-License-Identifier: AGPL-3.0-only
//! Independent-error and actual-time strip owners jointly retain a single clock-offset fibre.

use crate::lc_relation::{AT, calibration};
use crate::{Budget, LcQuantity as Q, LcRelation, LcRelationError, LcSampleTimes, LcSharedTiming};
use nrz_cas::Rational;

/// Effective sample coordinates omit physical alpha while retaining every original phase and scale.
pub(super) fn measured(
    timing: &LcSharedTiming,
    budget: &mut Budget,
) -> Result<LcSampleTimes, LcRelationError> {
    let mut inputs = Vec::new();
    for input in timing.relation().inputs() {
        budget.work(1, AT)?;
        if !matches!(input.quantity(), Q::Time(_) | Q::InitialPhase) {
            inputs.push(input.clone());
        }
    }
    for (index, (nominal, error)) in timing
        .nominal_times()
        .iter()
        .zip(timing.errors())
        .enumerate()
    {
        let time = [
            nominal.add(&error[0], budget)?,
            nominal.add(&error[1], budget)?,
        ];
        inputs.push(crate::lc_relation::timing::admit::input(
            Q::Time(index),
            time,
            budget,
        )?);
    }
    let relation = LcRelation::new(
        timing.relation().epoch().into(),
        timing.relation().observations(),
        inputs,
        budget,
    )?;
    LcSampleTimes::new(relation, timing.nominal_times().to_vec(), budget)
}

/// Exact original time intersections and the surviving common offset retain physical alpha.
pub(super) fn physical(
    timing: &LcSharedTiming,
    rate: &[Rational; 2],
    budget: &mut Budget,
) -> Result<LcSampleTimes, LcRelationError> {
    let mut inputs = Vec::new();
    for input in timing.relation().inputs() {
        budget.work(1, AT)?;
        if !matches!(input.quantity(), Q::Time(_)) {
            inputs.push(input.clone());
        }
    }
    inputs.push(crate::lc_relation::timing::admit::input(
        Q::Rate,
        rate.clone(),
        budget,
    )?);
    for (index, (nominal, error)) in timing
        .nominal_times()
        .iter()
        .zip(timing.errors())
        .enumerate()
    {
        let mut time = timing.time_boxes()[index].clone();
        let image = [
            nominal
                .add(&error[0], budget)?
                .add(&timing.offset_support()[0], budget)?,
            nominal
                .add(&error[1], budget)?
                .add(&timing.offset_support()[1], budget)?,
        ];
        calibration::intersect(&mut time, &image, budget)?;
        inputs.push(crate::lc_relation::timing::admit::input(
            Q::Time(index),
            time,
            budget,
        )?);
    }
    let relation = LcRelation::new(
        timing.relation().epoch().into(),
        timing.relation().observations(),
        inputs,
        budget,
    )?;
    LcSampleTimes::new(relation, timing.nominal_times().to_vec(), budget)
}

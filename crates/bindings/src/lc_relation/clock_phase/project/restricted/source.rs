// SPDX-License-Identifier: AGPL-3.0-only
//! Separate strip owners prove measured nonemptiness before exact physical reconstruction.

use crate::lc_relation::{AT, calibration, ordered};
use crate::{
    Budget, ExactConstant, LcInput, LcInterval, LcQuantity as Q, LcRelation, LcRelationError,
    LcSharedClock, LcSharedClockPhase,
};
use nrz_cas::Rational;

/// Original physical strips imply redundant measured constraints before independent complete-cycle checking.
pub(super) fn measured(
    phase: &LcSharedClockPhase,
    budget: &mut Budget,
) -> Result<LcRelation, LcRelationError> {
    let mut inputs = Vec::new();
    let offset = phase.offset().clone().map(ExactConstant::from);
    for value in phase.relation().inputs() {
        budget.work(1, AT)?;
        if !matches!(value.quantity(), Q::Phase(_) | Q::InitialPhase | Q::Time(_)) {
            inputs.push(value.clone());
        }
    }
    for (index, (time, sample)) in phase
        .clock()
        .nominal_times()
        .iter()
        .zip(phase.samples())
        .enumerate()
    {
        inputs.push(input(
            Q::Time(index),
            LcInterval::exact(time.clone().into(), budget)?,
            budget,
        )?);
        let clipped = measured_sample(phase, index, sample, &offset, budget)?;
        inputs.push(input(Q::Phase(index), clipped, budget)?);
    }
    LcRelation::new(
        phase.relation().epoch().into(),
        phase.relation().observations(),
        inputs,
        budget,
    )
}

/// Given a nonempty measured-intercept interval G, expanded strips are exactly G plus the shared S.
pub(super) fn physical(
    phase: &LcSharedClockPhase,
    rate: &[Rational; 2],
    budget: &mut Budget,
) -> Result<LcSharedClock, LcRelationError> {
    let mut inputs = phase.relation().inputs().to_vec();
    inputs.push(input(
        Q::Rate,
        calibration::interval(rate, budget)?,
        budget,
    )?);
    let offset = phase.offset().clone().map(ExactConstant::from);
    for (index, sample) in phase.samples().iter().enumerate() {
        budget.work(1, AT)?;
        let lower = sample
            .lower()
            .ok_or(LcRelationError::Projection)?
            .add(&offset[0], budget)?;
        let upper = sample
            .upper()
            .ok_or(LcRelationError::Projection)?
            .add(&offset[1], budget)?;
        inputs.push(input(
            Q::Phase(index),
            LcInterval::new(Some(lower), Some(upper), budget)?,
            budget,
        )?);
    }
    let relation = LcRelation::new(
        phase.relation().epoch().into(),
        phase.relation().observations(),
        inputs,
        budget,
    )?;
    LcSharedClock::new(
        relation,
        phase.clock().nominal_times().to_vec(),
        &calibration::interval(phase.clock().offset(), budget)?,
        budget,
    )
}

/// Every auxiliary constraint uses the original quantity's coherent descriptor.
fn input(
    quantity: Q,
    interval: LcInterval,
    budget: &mut Budget,
) -> Result<LcInput, LcRelationError> {
    LcInput::new(quantity, &quantity.descriptor()?, interval, budget)
}

/// Every original tuple implies `measured_i = physical_i-s` with the same declared `s`.
/// These redundant strips help prove complete cycle coverage; the original shared fibre remains retained.
fn measured_sample(
    phase: &LcSharedClockPhase,
    index: usize,
    sample: &LcInterval,
    offset: &[ExactConstant; 2],
    budget: &mut Budget,
) -> Result<LcInterval, LcRelationError> {
    let mut lower = sample.lower().cloned().ok_or(LcRelationError::Projection)?;
    let mut upper = sample.upper().cloned().ok_or(LcRelationError::Projection)?;
    for original in phase.relation().inputs() {
        budget.work(1, AT)?;
        if original.quantity() != Q::Phase(index) {
            continue;
        }
        let interval = original.interval();
        if let Some(bound) = interval.lower() {
            let candidate = bound.sub(&offset[1], budget)?;
            if ordered(&lower, &candidate, budget)? {
                lower = candidate;
            } else if !ordered(&candidate, &lower, budget)? {
                return Err(LcRelationError::UnprovedOrder);
            }
        }
        if let Some(bound) = interval.upper() {
            let candidate = bound.sub(&offset[0], budget)?;
            if ordered(&candidate, &upper, budget)? {
                upper = candidate;
            } else if !ordered(&upper, &candidate, budget)? {
                return Err(LcRelationError::UnprovedOrder);
            }
        }
    }
    LcInterval::new(Some(lower), Some(upper), budget)
}

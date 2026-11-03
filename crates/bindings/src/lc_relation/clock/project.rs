// SPDX-License-Identifier: AGPL-3.0-only
//! Shared clock and original initial phase form a convex strip problem in `beta=alpha+a*offset`.

use super::{
    AT, Cover, LcClockCover, LcClockPhaseParameters, LcInterval, LcRelation, LcRelationError,
    LcSharedClock, Rational, calibration, storage, witness,
};
use crate::{Budget, LcInput, LcPairsCover, LcQuantity as Q};
use nrz_cas::{
    limits::BudgetUsage,
    pi_bounds::{CheckedPiBounds, PiBoundCertificate},
};
use std::rc::Rc;

/// Two redundant finite strip faces encode the exact shared-offset inequalities.
pub(super) fn cover(
    clock: &LcSharedClock,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcClockCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let (relation, cycles) = prepare(clock, cycles, budget)?;
    let effective = relation.pairs_cover(cycles, pi, budget)?;
    finish(clock, &relation, effective, &before, budget)
}

/// Previously checked pi evidence is shared across the measured and physical strip owners.
pub(super) fn checked(
    clock: &LcSharedClock,
    cycles: Vec<Rational>,
    pi: CheckedPiBounds,
    budget: &mut Budget,
) -> Result<LcClockCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let (relation, cycles) = prepare(clock, cycles, budget)?;
    let effective = relation.pairs_cover_checked(cycles, pi, budget)?;
    finish(clock, &relation, effective, &before, budget)
}

/// Original-cycle inventory and derived clock faces precede either prepared-constant route.
fn prepare(
    clock: &LcSharedClock,
    mut cycles: Vec<Rational>,
    budget: &mut Budget,
) -> Result<(LcRelation, Vec<Rational>), LcRelationError> {
    if cycles.len() != clock.relation().observations() {
        return Err(LcRelationError::Observation);
    }
    let relation = effective(clock, budget)?;
    if clock.initial_phase_support().is_some() {
        cycles.extend([Rational::zero(), Rational::zero()]);
    }
    Ok((relation, cycles))
}

/// Both constant routes share exact physical restoration and complete retained ownership.
fn finish(
    clock: &LcSharedClock,
    relation: &LcRelation,
    effective: LcPairsCover,
    before: &BudgetUsage,
    budget: &mut Budget,
) -> Result<LcClockCover, LcRelationError> {
    let rate = effective.rate()[0].clone();
    let phase = effective.phase_at_rate(&rate, budget)?;
    let mut offset = clock.offset_support().clone();
    if let Some([lower, upper]) = clock.initial_phase_support() {
        let image = [
            phase[0].sub(upper, budget)?.div(&rate, budget)?,
            phase[0].sub(lower, budget)?.div(&rate, budget)?,
        ];
        calibration::intersect(&mut offset, &image, budget)?;
    }
    let parameters = LcClockPhaseParameters {
        initial_phase: phase[0].sub(&rate.mul(&offset[0], budget)?, budget)?,
        offset: offset[0].clone(),
        gauge: Rational::zero(),
        rate,
        ratio: Rational::one(),
        voltage: Rational::one(),
    };
    let witness = witness::with_phase(clock, &effective, &parameters, budget)?;
    let bytes = effective
        .retained_payload_bytes()
        .saturating_add(relation.retained_payload_bytes())
        .saturating_add(clock.retained_payload_bytes())
        .saturating_add(witness.original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.offset()))
        .saturating_add(2_048);
    storage::retained(
        bytes.saturating_add(clock.relation().retained_payload_bytes()),
        budget,
    )?;
    Ok(LcClockCover(Rc::new(Cover {
        clock: clock.clone(),
        effective,
        witness,
        usage: [*before, budget.usage()],
        bytes,
    })))
}

/// The original physical initial phase remains correlated through `beta=alpha+a*offset`.
fn effective(clock: &LcSharedClock, budget: &mut Budget) -> Result<LcRelation, LcRelationError> {
    let count = clock.relation().observations();
    let auxiliary = if clock.initial_phase_support().is_some() {
        2
    } else {
        0
    };
    let total = count
        .checked_add(auxiliary)
        .ok_or(LcRelationError::Capacity)?;
    storage::retained(
        clock
            .relation()
            .retained_payload_bytes()
            .saturating_mul(2)
            .saturating_add(32_768),
        budget,
    )?;
    let mut inputs = Vec::new();
    for input in clock.relation().inputs() {
        budget.work(1, AT)?;
        if !matches!(input.quantity(), Q::Time(_) | Q::InitialPhase) {
            inputs.push(input.clone());
        }
    }
    for (index, time) in clock.nominal_times().iter().enumerate() {
        inputs.push(input(Q::Time(index), [time.clone(), time.clone()], budget)?);
    }
    if let Some(initial) = clock.initial_phase_support() {
        initial_strips(clock, initial, &mut inputs, budget)?;
    }
    LcRelation::new(
        format!(
            "{}: effective shared-clock coordinates",
            clock.relation().epoch()
        ),
        total,
        inputs,
        budget,
    )
}

/// Bounded original initial phase and clock support give two exact affine strip inequalities.
fn initial_strips(
    clock: &LcSharedClock,
    initial: &[Rational; 2],
    inputs: &mut Vec<LcInput>,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let count = clock.relation().observations();
    let rate = &clock.0.rate;
    let offset = clock.offset_support();
    let low_rate = if offset[0].is_negative() {
        &rate[1]
    } else {
        &rate[0]
    };
    let high_rate = if offset[1].is_negative() {
        &rate[0]
    } else {
        &rate[1]
    };
    inputs.push(input(
        Q::InitialPhase,
        [
            initial[0].add(&low_rate.mul(&offset[0], budget)?, budget)?,
            initial[1].add(&high_rate.mul(&offset[1], budget)?, budget)?,
        ],
        budget,
    )?);
    let span = rate[1].mul(&offset[1].sub(&offset[0], budget)?, budget)?;

    // `beta-a*d_min >= A_min` and `beta-a*d_max <= A_max` are exactly the shared-offset domain.
    // Their opposite finite faces are redundant because `a<=a_max` and `d_min<=d_max`.
    for (index, duration, phase) in [
        (
            count,
            &offset[0],
            [initial[0].clone(), initial[1].add(&span, budget)?],
        ),
        (
            count + 1,
            &offset[1],
            [initial[0].sub(&span, budget)?, initial[1].clone()],
        ),
    ] {
        let time = Rational::zero().sub(duration, budget)?;
        inputs.push(input(Q::Time(index), [time.clone(), time], budget)?);
        inputs.push(input(Q::Phase(index), phase, budget)?);
    }
    Ok(())
}

/// Physical coordinate declarations retain exact coherent meanings in the effective relation.
fn input(
    quantity: Q,
    bounds: [Rational; 2],
    budget: &mut Budget,
) -> Result<LcInput, LcRelationError> {
    let [lower, upper] = bounds;
    let interval = LcInterval::new(Some(lower.into()), Some(upper.into()), budget)?;
    LcInput::new(quantity, &quantity.descriptor()?, interval, budget)
}

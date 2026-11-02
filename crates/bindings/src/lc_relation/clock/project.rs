// SPDX-License-Identifier: AGPL-3.0-only
//! Positive clock elimination is a convex strip problem in `beta=a*offset`.

use super::{
    AT, Cover, LcClockCover, LcClockParameters, LcInterval, LcRelation, LcRelationError,
    LcSharedClock, Rational, storage, witness,
};
use crate::{Budget, LcInput, LcQuantity as Q};
use nrz_cas::pi_bounds::PiBoundCertificate;
use std::rc::Rc;

/// Two redundant finite strip faces encode the exact shared-offset inequalities.
pub(super) fn cover(
    clock: &LcSharedClock,
    mut cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcClockCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    if cycles.len() != clock.relation().observations() {
        return Err(LcRelationError::Observation);
    }
    let relation = effective(clock, budget)?;
    cycles.extend([Rational::zero(), Rational::zero()]);
    let effective = relation.pairs_cover(cycles, pi, budget)?;
    let rate = effective.rate()[0].clone();
    let phase = effective.phase_at_rate(&rate, budget)?;
    let parameters = LcClockParameters {
        offset: phase[0].div(&rate, budget)?,
        rate,
        ratio: Rational::one(),
        voltage: Rational::one(),
    };
    let witness = witness::parameters(clock, &effective, &parameters, budget)?;
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
        usage: [before, budget.usage()],
        bytes,
    })))
}

/// The original known-zero phase becomes the bounded effective starting phase `beta=a*offset`.
fn effective(clock: &LcSharedClock, budget: &mut Budget) -> Result<LcRelation, LcRelationError> {
    let count = clock.relation().observations();
    let total = count.checked_add(2).ok_or(LcRelationError::Capacity)?;
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
    let rate = &clock.0.rate;
    let offset = clock.offset();
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
            low_rate.mul(&offset[0], budget)?,
            high_rate.mul(&offset[1], budget)?,
        ],
        budget,
    )?);
    let span = rate[1].mul(&offset[1].sub(&offset[0], budget)?, budget)?;
    let negative_span = Rational::zero().sub(&span, budget)?;
    // `beta-a*d_min >= 0` and `beta-a*d_max <= 0` are exactly the shared-offset domain.
    // Their opposite finite faces are redundant because `a<=a_max` and `d_min<=d_max`.
    for (index, duration, phase) in [
        (count, &offset[0], [Rational::zero(), span]),
        (count + 1, &offset[1], [negative_span, Rational::zero()]),
    ] {
        let time = Rational::zero().sub(duration, budget)?;
        inputs.push(input(Q::Time(index), [time.clone(), time], budget)?);
        inputs.push(input(Q::Phase(index), phase, budget)?);
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

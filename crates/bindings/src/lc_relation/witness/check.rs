// SPDX-License-Identifier: AGPL-3.0-only
//! Original positivity, energy and lifted phase equations are checked before input membership.

use super::{AT, Derived, LcBranch, LcRelationError, LcValues};
use crate::{Budget, ExactConstant};
use nrz_cas::{Rational, syntax::Constant};

/// Check the original laws before constructing any point-coordinate projection.
pub(super) fn original(
    values: &LcValues,
    budget: &mut Budget,
) -> Result<(LcBranch, Derived), LcRelationError> {
    for value in [&values.inductance, &values.capacitance, &values.rate] {
        positive(value, budget)?;
    }
    let product = values.inductance.mul(&values.capacitance, budget)?;
    equal(
        &values.rate.pow(2, budget)?.mul(&product, budget)?,
        &ExactConstant::one(),
        budget,
    )?;
    let natural_time = ExactConstant::one().div(&values.rate, budget)?;
    let two = ExactConstant::from(Rational::from_ratio(2, 1, budget)?);
    let twice_energy = two.mul(&values.energy, budget)?;
    for (element, amplitude) in [
        (&values.capacitance, &values.voltage),
        (&values.inductance, &values.current),
    ] {
        equal(
            &element.mul(&amplitude.pow(2, budget)?, budget)?,
            &twice_energy,
            budget,
        )?;
    }
    let (branch, ratio) = if values.energy.is_zero() {
        quiescent(values, budget)?;
        (LcBranch::Quiescent, None)
    } else {
        for value in [&values.energy, &values.voltage, &values.current] {
            positive(value, budget)?;
        }
        phases(values, &two, budget)?;
        let ratio = values.voltage.div(&values.current, budget)?;
        // Retain the redundant amplitude equation as a checked original constraint.
        equal(
            &ratio.pow(2, budget)?,
            &values.inductance.div(&values.capacitance, budget)?,
            budget,
        )?;
        (LcBranch::Oscillating, Some(ratio))
    };
    Ok((
        branch,
        Derived {
            product,
            natural_time,
            ratio,
        },
    ))
}

/// Zero energy cannot manufacture a ratio or an arbitrary measured phase clock.
fn quiescent(values: &LcValues, budget: &mut Budget) -> Result<(), LcRelationError> {
    if !values.voltage.is_zero() || !values.current.is_zero() {
        return Err(LcRelationError::Equation);
    }
    if values.initial_phase.is_some() {
        return Err(LcRelationError::QuiescentPhase);
    }
    for value in &values.observations {
        budget.work(1, AT)?;
        if value.phase.is_some() || value.cycle.is_some() {
            return Err(LcRelationError::QuiescentPhase);
        }
    }
    Ok(())
}

/// Every observed phase retains its exact integer lift; there is no finite alias search here.
fn phases(
    values: &LcValues,
    two: &ExactConstant,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    let initial = values
        .initial_phase
        .as_ref()
        .ok_or(LcRelationError::Observation)?;
    let turn = ExactConstant::named(Constant::Pi, budget)?.mul(two, budget)?;
    for observation in &values.observations {
        budget.work(1, AT)?;
        let phase = observation
            .phase
            .as_ref()
            .ok_or(LcRelationError::Observation)?;
        let cycle = observation
            .cycle
            .as_ref()
            .ok_or(LcRelationError::Observation)?;
        cycle.check(budget, AT)?;
        if !cycle.is_integer() {
            return Err(LcRelationError::Cycle);
        }
        let lifted = initial.add(&values.rate.mul(&observation.time, budget)?, budget)?;
        let measured = phase.add(
            &turn.mul(&ExactConstant::from(cycle.clone()), budget)?,
            budget,
        )?;
        equal(&lifted, &measured, budget)?;
    }
    Ok(())
}

/// Positive exact signs are checked facts, never inferred from a center or label.
fn positive(value: &ExactConstant, budget: &mut Budget) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    if value.positive().is_none() {
        return Err(LcRelationError::Positivity);
    }
    Ok(())
}

/// A finite exact identity uses the CAS normal form under the caller's ledger.
fn equal(
    left: &ExactConstant,
    right: &ExactConstant,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    if !left.structurally_equal(right, budget)? {
        return Err(LcRelationError::Equation);
    }
    Ok(())
}

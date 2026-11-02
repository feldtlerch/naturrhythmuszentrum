// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact original assignments and deterministic observation constraints.

use nrz_bindings::{
    Budget, ExactConstant, LcInput, LcInterval, LcObservationValue, LcQuantity, LcRelation,
    LcValues, Limits,
};
use nrz_cas::{Rational, syntax::Constant};

/// Tests propagate original typed failures without panicking through helper code.
pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Existing defaults remain unchanged.
pub(super) fn ledger() -> TestResult<Budget> {
    Ok(Budget::new(Limits::default())?)
}

/// Parse an independent exact rational coordinate.
pub(super) fn number(text: &str) -> TestResult<ExactConstant> {
    Ok(Rational::parse(text, &mut ledger()?)?.into())
}

/// A cycle is a mathematical integer, not a floating coordinate.
pub(super) fn integer(value: i64) -> TestResult<Rational> {
    Ok(Rational::from_ratio(value, 1, &mut ledger()?)?)
}

/// Original phase center 32/5 - 2 pi is never replaced by a rounded decimal.
fn final_phase() -> TestResult<ExactConstant> {
    let mut budget = ledger()?;
    let pi = ExactConstant::named(Constant::Pi, &mut budget)?;
    Ok(number("32/5")?.sub(&number("2")?.mul(&pi, &mut budget)?, &mut budget)?)
}

/// A complete exact nominal tuple, independently specified in coherent SI coordinates.
pub(super) fn reference() -> TestResult<LcValues> {
    let mut observations = Vec::new();
    for (time, phase, cycle) in [
        ("1/1000", number("2")?, 0),
        ("1/400", number("5")?, 0),
        ("2/625", final_phase()?, 1),
    ] {
        observations.push(LcObservationValue {
            time: number(time)?,
            phase: Some(phase),
            cycle: Some(integer(cycle)?),
        });
    }
    Ok(LcValues {
        inductance: number("1/40")?,
        capacitance: number("1/100000")?,
        rate: number("2000")?,
        initial_phase: Some(number("0")?),
        voltage: number("5")?,
        current: number("1/10")?,
        energy: number("1/8000")?,
        observations,
    })
}

/// Rest preserves the same positive elements and natural frequency.
pub(super) fn quiescent() -> TestResult<LcValues> {
    let mut values = reference()?;
    values.initial_phase = None;
    values.voltage = number("0")?;
    values.current = number("0")?;
    values.energy = number("0")?;
    values.observations.clear();
    Ok(values)
}

/// Admit declared exact endpoints under the required original physical meaning.
pub(super) fn input(
    quantity: LcQuantity,
    lower: Option<ExactConstant>,
    upper: Option<ExactConstant>,
) -> TestResult<LcInput> {
    let interval = LcInterval::new(lower, upper, &mut ledger()?)?;
    Ok(LcInput::new(
        quantity,
        &quantity.descriptor()?,
        interval,
        &mut ledger()?,
    )?)
}

/// Exact constraints remain singleton intervals, including exact irrational phase centers.
pub(super) fn exact_input(quantity: LcQuantity, value: ExactConstant) -> TestResult<LcInput> {
    input(quantity, Some(value.clone()), Some(value))
}

/// Every fixture time is measured from the same explicitly named epoch.
pub(super) fn relation(inputs: Vec<LcInput>) -> TestResult<LcRelation> {
    Ok(LcRelation::new(
        "measurement epoch".into(),
        3,
        inputs,
        &mut ledger()?,
    )?)
}

/// Original rational timestamps and rational-width boxes around exact phase centers.
pub(super) fn observations(radius: &str) -> TestResult<Vec<LcInput>> {
    let mut inputs = Vec::new();
    let radius = number(radius)?;
    for (i, value) in reference()?.observations.into_iter().enumerate() {
        let phase = value.phase.ok_or("reference phase")?;
        let mut budget = ledger()?;
        inputs.push(exact_input(LcQuantity::Time(i), value.time)?);
        inputs.push(input(
            LcQuantity::Phase(i),
            Some(phase.sub(&radius, &mut budget)?),
            Some(phase.add(&radius, &mut budget)?),
        )?);
    }
    Ok(inputs)
}

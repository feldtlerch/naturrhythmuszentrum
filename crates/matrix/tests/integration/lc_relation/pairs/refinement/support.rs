// SPDX-License-Identifier: AGPL-3.0-only
//! Original input tightening and added observations are generated independently of the solver.

use super::{TestResult, ledger, number, pairs};
use nrz_bindings::readings::algebra::Rational;
use nrz_bindings::{LcInput, LcInterval, LcQuantity as Q, LcRelation};
use std::cmp::Ordering;

/// Independent exact subset checks use only arbitrary-precision endpoint comparisons.
pub(super) fn contained(previous: &[Rational; 2], next: &[Rational; 2]) -> TestResult {
    assert_ne!(
        previous[0].compare(&next[0], &mut ledger()?)?,
        Ordering::Greater
    );
    assert_ne!(
        next[1].compare(&previous[1], &mut ledger()?)?,
        Ordering::Greater
    );
    Ok(())
}

/// Keep the original clock and branch; an added later observation has center eight radians lifted.
pub(super) fn refined(known: bool, radius: &str, added: bool) -> TestResult<LcRelation> {
    const LAST: usize = 2;
    let mut inputs = pairs::inputs(known, radius)?;
    if added {
        append(&mut inputs, LAST, "1/250", "8/5")?;
    }
    Ok(LcRelation::new(
        "recovery epoch".into(),
        if added { 4 } else { 3 },
        inputs,
        &mut ledger()?,
    )?)
}

/// An additional same-time observation differs by two fifths of a radian on every lift.
pub(super) fn impossible_inputs() -> TestResult<LcRelation> {
    const FIRST: usize = 0;
    let mut inputs = pairs::inputs(true, "1/20")?;
    append(&mut inputs, FIRST, "1/1000", "2/5")?;
    Ok(LcRelation::new(
        "recovery epoch".into(),
        4,
        inputs,
        &mut ledger()?,
    )?)
}

/// Reuse the original exact phase constant instead of preparing the same turn again.
fn append(inputs: &mut Vec<LcInput>, source: usize, time: &str, offset: &str) -> TestResult {
    const ADDED: usize = 3;
    let phase = inputs
        .iter()
        .find(|input| input.quantity() == Q::Phase(source))
        .ok_or("source phase")?
        .interval();
    let offset = nrz_bindings::ExactConstant::from(number(offset)?);
    let interval = LcInterval::new(
        Some(
            phase
                .lower()
                .ok_or("phase lower")?
                .add(&offset, &mut ledger()?)?,
        ),
        Some(
            phase
                .upper()
                .ok_or("phase upper")?
                .add(&offset, &mut ledger()?)?,
        ),
        &mut ledger()?,
    )?;
    inputs.push(LcInput::new(
        Q::Phase(ADDED),
        &Q::Phase(ADDED).descriptor()?,
        interval,
        &mut ledger()?,
    )?);
    inputs.push(super::super::super::support::input(
        Q::Time(ADDED),
        [time, time],
    )?);
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Exact original measurement questions used by whole-clock refinement fixtures.

use super::{TestResult, input, ledger, number};
use nrz_bindings::{
    LcClockPhaseParameters, LcHorizon, LcHorizonValue, LcInterval, LcQuantity as Q, LcRelation,
    LcSharedClock, LcSharedTiming,
};

/// Original phase strips narrow while physical time boxes and observation identities remain unchanged.
pub(super) fn clock(known: bool, width: &str, offset: [&str; 2]) -> TestResult<LcSharedClock> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    let width = number(width)?;
    for (index, (time, phase, multiple)) in
        [(["4/5", "6/5"], "2", "1"), (["9/5", "11/5"], "4", "2")]
            .into_iter()
            .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        let centre = number(phase)?;
        let radius = width.mul(&number(multiple)?, &mut ledger()?)?;
        let bounds = [
            centre.sub(&radius, &mut ledger()?)?,
            centre.add(&radius, &mut ledger()?)?,
        ];
        inputs.push(nrz_bindings::LcInput::new(
            Q::Phase(index),
            &Q::Phase(index).descriptor()?,
            LcInterval::new(
                Some(bounds[0].clone().into()),
                Some(bounds[1].clone().into()),
                &mut ledger()?,
            )?,
            &mut ledger()?,
        )?);
    }
    let relation = LcRelation::new(
        "shared clock refinement epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &interval(offset)?,
        &mut ledger()?,
    )?)
}

/// Both declared time-error layers remain explicit when original timestamps narrow.
pub(super) fn timing(
    known: bool,
    narrow: bool,
    smaller_errors: bool,
) -> TestResult<LcSharedTiming> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    let times = if narrow {
        [["99/100", "101/100"], ["199/100", "201/100"]]
    } else {
        [["89/100", "111/100"], ["47/25", "53/25"]]
    };
    for (index, (time, phase)) in times.into_iter().zip(["2", "4"]).enumerate() {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    let relation = LcRelation::new(
        "two-layer refinement epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let errors = if smaller_errors {
        [["-1/200", "1/200"], ["-1/100", "1/100"]]
    } else {
        [["-1/100", "1/100"], ["-1/50", "1/50"]]
    };
    Ok(LcSharedTiming::new(
        relation,
        vec![number("1")?, number("2")?],
        &errors
            .map(interval)
            .into_iter()
            .collect::<TestResult<Vec<_>>>()?,
        &interval(if narrow {
            ["-1/20", "1/20"]
        } else {
            ["-1/10", "1/10"]
        })?,
        &mut ledger()?,
    )?)
}

/// Original physical and shared coordinates are always reconstructed together.
pub(super) fn parameters(
    rate: nrz_cas::Rational,
    phase: nrz_cas::Rational,
    offset: nrz_cas::Rational,
) -> LcClockPhaseParameters {
    LcClockPhaseParameters {
        rate,
        initial_phase: phase,
        offset,
        gauge: nrz_cas::Rational::zero(),
        ratio: nrz_cas::Rational::one(),
        voltage: nrz_cas::Rational::one(),
    }
}

/// Finite original signed measurement supports use exact endpoints.
pub(super) fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Independent exact affine coefficients identify the no-earlier physical threshold.
pub(super) fn finite(horizon: &LcHorizon, offset: &str, coefficient: &str) -> TestResult {
    let LcHorizonValue::Finite(value) = horizon.value() else {
        return Err("expected finite refined horizon".into());
    };
    assert_eq!(
        value.coefficients(),
        (&number(offset)?, &number(coefficient)?)
    );
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact rational timing fixtures use existing arbitrary-precision arithmetic.

use super::super::support::{TestResult, ledger};
use nrz_bindings::{
    LcInput, LcInterval, LcObservationValue, LcQuantity as Q, LcRelation, LcSampleTimes, LcValues,
};
use nrz_cas::{Rational, pi_bounds::PiBoundCertificate};

/// Exact rationals refuse on exhausted bounds rather than overflowing native arithmetic.
pub(super) fn number(text: &str) -> TestResult<Rational> {
    Ok(Rational::parse(text, &mut ledger()?)?)
}

/// Original physical input, including repeated declarations when requested by a control.
pub(super) fn input(quantity: Q, range: [&str; 2]) -> TestResult<LcInput> {
    let interval = LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?;
    Ok(LcInput::new(
        quantity,
        &quantity.descriptor()?,
        interval,
        &mut ledger()?,
    )?)
}

/// Unequal time supports with exact phase observations and known zero initial phase.
pub(super) fn original() -> TestResult<LcRelation> {
    let mut inputs = vec![
        input(Q::Rate, ["1", "3"])?,
        input(Q::InitialPhase, ["0", "0"])?,
    ];
    for (index, (time, phase)) in [(["9/10", "11/10"], "2"), (["9/5", "23/10"], "4")]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    Ok(LcRelation::new(
        "independent sample epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?)
}

/// Nominal values define separate errors without changing the physical input relation.
pub(super) fn sample_times() -> TestResult<LcSampleTimes> {
    Ok(LcSampleTimes::new(
        original()?,
        vec![number("1")?, number("2")?],
        &mut ledger()?,
    )?)
}

/// Both observations use the same declared zero-cycle representative.
pub(super) fn cycles() -> Vec<Rational> {
    vec![Rational::zero(), Rational::zero()]
}

/// Independently specified Machin certificate; the production kernel checks its remainder.
pub(super) fn pi() -> TestResult<PiBoundCertificate> {
    Ok(PiBoundCertificate {
        version: 1,
        terms: [2, 1],
        sums: [number("74/375")?, number("1/239")?],
        lower: number("31/10")?,
        upper: number("22/7")?,
    })
}

/// Original equations with unit impedance and amplitudes, not a relaxed solver witness.
pub(super) fn point(rate: &str, times: [&str; 2]) -> TestResult<LcValues> {
    let rate = number(rate)?;
    let element = Rational::one().div(&rate, &mut ledger()?)?;
    let energy = element.div(&number("2")?, &mut ledger()?)?;
    let observations = times
        .into_iter()
        .zip(["2", "4"])
        .map(|(time, phase)| {
            Ok(LcObservationValue {
                time: number(time)?.into(),
                phase: Some(number(phase)?.into()),
                cycle: Some(Rational::zero()),
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(LcValues {
        inductance: element.clone().into(),
        capacitance: element.into(),
        rate: rate.into(),
        initial_phase: Some(number("0")?.into()),
        voltage: number("1")?.into(),
        current: number("1")?.into(),
        energy: energy.into(),
        observations,
    })
}

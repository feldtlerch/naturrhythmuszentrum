// SPDX-License-Identifier: AGPL-3.0-only
//! Original clock, phase and amplitude measurements with independent physical equation oracles.

use super::{TestResult, input, ledger, number};
use nrz_bindings::{
    Budget, LcInput, LcInterval, LcQuantity as Q, LcRelation, LcRelationError, LcSharedClock,
    LcSharedClockPhase, LcSharedMeasurementParameters, LcSharedMeasurementWitness,
    LcSharedMeasurements, Limits, LimitsOverride,
};

/// One original relation contains all physical inputs; every measurement retains its own layer.
pub(super) fn source(
    channel: Q,
    known_phase: bool,
    energy: bool,
    extra: Vec<LcInput>,
) -> TestResult<LcSharedClockPhase> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?, input(channel, ["1/2", "4"])?];
    if known_phase {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    if energy {
        inputs.push(input(Q::Energy, ["1/4", "1/4"])?);
    }
    for (index, (time, phase)) in [
        (["9/10", "11/10"], ["17/10", "23/10"]),
        (["19/10", "21/10"], ["37/10", "43/10"]),
    ]
    .into_iter()
    .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    inputs.extend(extra);
    let relation = LcRelation::new(
        "three shared errors epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let clock = LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?;
    Ok(LcSharedClockPhase::new(
        clock,
        vec![interval(["2", "2"])?, interval(["4", "4"])?],
        &interval(["-1/5", "1/5"])?,
        &mut ledger()?,
    )?)
}

/// Amplitude readings and calibration can be removed separately without changing the original question.
pub(super) fn question(
    channel: Q,
    [per_sample, shared_gain, known_phase, energy]: [bool; 4],
) -> TestResult<LcSharedMeasurements> {
    let phase = source(channel, known_phase, energy, Vec::new())?;
    let ranges = if per_sample {
        [["9/10", "11/10"], ["19/20", "23/20"]]
    } else {
        [["1", "1"], ["1", "1"]]
    };
    let samples = ranges
        .into_iter()
        .map(interval)
        .collect::<TestResult<Vec<_>>>()?;
    let gain = if shared_gain {
        ["9/10", "11/10"]
    } else {
        ["1", "1"]
    };
    Ok(LcSharedMeasurements::new(
        phase,
        channel,
        samples,
        &interval(gain)?,
        &mut ledger()?,
    )?)
}

/// Exact rational intervals carry coherent dimensions at their associated owner.
pub(super) fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Every systematic coordinate is chosen on the same original physical tuple.
pub(super) fn parameters(
    initial: &str,
    clock: &str,
    phase: &str,
    measured: &str,
    gain: &str,
    gauge: &str,
) -> TestResult<LcSharedMeasurementParameters> {
    Ok(LcSharedMeasurementParameters {
        rate: number("2")?,
        initial_phase: number(initial)?,
        gauge: number(gauge)?,
        clock_offset: number(clock)?,
        phase_offset: number(phase)?,
        measured: number(measured)?,
        gain: number(gain)?,
        energy: number("1/4")?,
    })
}

/// Public endpoints agree with independently derived literal rational extrema.
pub(super) fn assert_bounds(actual: &LcInterval, expected: [&str; 2]) -> TestResult {
    assert_eq!(actual.lower(), Some(&number(expected[0])?.into()));
    assert_eq!(actual.upper(), Some(&number(expected[1])?.into()));
    Ok(())
}

/// Independent arbitrary-precision equations check the full restored physical scales.
pub(super) fn check_physical(
    source: &LcSharedMeasurements,
    point: &LcSharedMeasurementWitness,
) -> TestResult {
    let original = point.original().original().original();
    assert!(original.relation().shares_allocation(source.relation()));
    let values = original.values();
    let inductance = values.inductance.as_rational().ok_or("rational L")?;
    let capacitance = values.capacitance.as_rational().ok_or("rational C")?;
    let rate = values.rate.as_rational().ok_or("rational rate")?;
    let voltage = values.voltage.as_rational().ok_or("rational V")?;
    let current = values.current.as_rational().ok_or("rational I")?;
    let energy = values.energy.as_rational().ok_or("rational E")?;
    let mut budget = ledger()?;
    assert_eq!(
        inductance
            .mul(capacitance, &mut budget)?
            .mul(&rate.mul(rate, &mut budget)?, &mut budget)?,
        number("1")?
    );
    assert_eq!(
        voltage.div(current, &mut budget)?,
        rate.mul(inductance, &mut budget)?
    );
    assert_eq!(
        capacitance.mul(&voltage.mul(voltage, &mut budget)?, &mut budget)?,
        number("2")?.mul(energy, &mut budget)?
    );
    assert_eq!(
        inductance.mul(&current.mul(current, &mut budget)?, &mut budget)?,
        number("2")?.mul(energy, &mut budget)?
    );
    source.check_witness(
        values.clone(),
        point.original().original().offset().clone(),
        point.original().offset().clone(),
        point.gain().clone(),
        &mut ledger()?,
    )?;
    Ok(())
}

/// Unmodeled physical scales and disjoint original amplitude restrictions refuse explicitly.
pub(super) fn original_refusals() -> TestResult {
    for quantity in [
        Q::Inductance,
        Q::Capacitance,
        Q::AmplitudeRatio,
        Q::CurrentAmplitude,
        Q::VoltageAmplitude,
    ] {
        let range = if quantity == Q::VoltageAmplitude {
            ["2", "3"]
        } else {
            ["1", "1"]
        };
        let phase = source(
            Q::VoltageAmplitude,
            true,
            true,
            vec![input(quantity, range)?],
        )?;
        assert!(
            LcSharedMeasurements::new(
                phase,
                Q::VoltageAmplitude,
                vec![interval(["9/10", "11/10"])?],
                &interval(["9/10", "11/10"])?,
                &mut ledger()?
            )
            .is_err()
        );
    }
    Ok(())
}

/// Adjacent cumulative Work allowances prove success and typed exhaustion without raising a default.
pub(super) fn work_boundary<T>(
    mut operation: impl FnMut(&mut Budget) -> Result<T, LcRelationError>,
) -> TestResult {
    let mut measured = ledger()?;
    assert!(operation(&mut measured).is_ok());
    let work = measured.usage().work;
    assert!(work > 0);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?)?;
        budget.work(17, nrz_cas::Location::new("three shared errors prefix"))?;
        let outcome = operation(&mut budget);
        assert_eq!(outcome.is_ok(), succeeds);
        if !succeeds {
            assert!(matches!(
                outcome,
                Err(LcRelationError::Algebra(nrz_cas::Fault {
                    kind: nrz_cas::FaultKind::Exhausted {
                        budget: nrz_cas::limits::BudgetKind::Work,
                        ..
                    },
                    ..
                }))
            ));
        }
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact element fixtures and physical equation oracles.

use super::{TestResult, input, ledger, number};
use nrz_bindings::{
    Budget, LcElementParameters, LcElementSamples, LcElementWitness, LcInput, LcInterval,
    LcQuantity as Q, LcRelation, LcRelationError, LcSampleTimes, Limits, LimitsOverride,
};

/// Original phase observations may retain unequal timestamp intervals and unknown initial phase.
pub(super) fn timing(
    uncertain: bool,
    known_phase: bool,
    extra: Vec<LcInput>,
) -> TestResult<LcSampleTimes> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known_phase {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    let times = if uncertain {
        [["9/10", "11/10"], ["9/5", "23/10"]]
    } else {
        [["1", "1"], ["2", "2"]]
    };
    for (index, (time, phase)) in times.into_iter().zip(["2", "4"]).enumerate() {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    inputs.extend(extra);
    let relation = LcRelation::new(
        "element measurement epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSampleTimes::new(
        relation,
        vec![number("1")?, number("2")?],
        &mut ledger()?,
    )?)
}

/// Coherent finite rational measurement supports.
pub(super) fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Separate independent readings can be replaced by exact unit readings for the shared-only control.
pub(super) fn readings(per_sample: bool) -> TestResult<Vec<LcInterval>> {
    let ranges = if per_sample {
        [["9/10", "11/10"], ["19/20", "23/20"]]
    } else {
        [["1", "1"], ["1", "1"]]
    };
    ranges.into_iter().map(interval).collect()
}

/// Retain the original timing owner and one inductance calibration.
pub(super) fn measured(
    timing: LcSampleTimes,
    per_sample: bool,
    shared: bool,
) -> TestResult<LcElementSamples> {
    let gain = if shared {
        ["9/10", "11/10"]
    } else {
        ["1", "1"]
    };
    Ok(LcElementSamples::new(
        timing,
        Q::Inductance,
        readings(per_sample)?,
        &interval(gain)?,
        &mut ledger()?,
    )?)
}

/// Each coordinate is chosen explicitly; no combination of separate physical marginals is inferred.
pub(super) fn parameters(
    rate: &str,
    phase: &str,
    element: &str,
    gain: &str,
    voltage: &str,
    gauge: &str,
) -> TestResult<LcElementParameters> {
    Ok(LcElementParameters {
        rate: number(rate)?,
        initial_phase: number(phase)?,
        gauge: number(gauge)?,
        element: number(element)?,
        calibration: number(gain)?,
        voltage: number(voltage)?,
    })
}

/// Independent literal rational endpoints are compared through the public interval surface.
pub(super) fn assert_bounds(actual: &LcInterval, expected: [&str; 2]) -> TestResult {
    assert_eq!(actual.lower(), Some(&number(expected[0])?.into()));
    assert_eq!(actual.upper(), Some(&number(expected[1])?.into()));
    Ok(())
}

/// Arbitrary-precision physical equations independently check all reconstructed scales.
pub(super) fn check_physical(element: &LcElementSamples, point: &LcElementWitness) -> TestResult {
    let values = point.original().values();
    let inductance = values.inductance.as_rational().ok_or("rational L")?;
    let capacitance = values.capacitance.as_rational().ok_or("rational C")?;
    let rate = values.rate.as_rational().ok_or("rational rate")?;
    let voltage = values.voltage.as_rational().ok_or("rational voltage")?;
    let current = values.current.as_rational().ok_or("rational current")?;
    let energy = values.energy.as_rational().ok_or("rational energy")?;
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
    element.check_witness(values.clone(), point.calibration().clone(), &mut ledger()?)?;
    Ok(())
}

/// Adjacent remaining Work allowances establish success and a typed Work exhaustion refusal.
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
        budget.work(17, nrz_cas::Location::new("element calibration prefix"))?;
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

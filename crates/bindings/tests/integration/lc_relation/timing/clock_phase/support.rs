// SPDX-License-Identifier: AGPL-3.0-only
//! Original common clock/phase fixtures with independent exact measurements.

use super::{TestResult, cycles, input, ledger, number, pi};
use nrz_bindings::{
    Budget, LcClockPhaseParameters, LcInterval, LcQuantity as Q, LcRelation, LcRelationError,
    LcSharedClock, LcSharedClockPhase, LcSharedClockPhaseParameters, Limits, LimitsOverride,
};

/// The same original physical boxes contain every requested systematic-error image.
pub(super) fn question(
    [clock_shared, phase_shared, known_phase, per_sample]: [bool; 4],
) -> TestResult<LcSharedClockPhase> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known_phase {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
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
    let relation = LcRelation::new(
        "combined shared errors epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let clock_offset = if clock_shared {
        ["-1/10", "1/10"]
    } else {
        ["0", "0"]
    };
    let phase_offset = if phase_shared {
        ["-1/5", "1/5"]
    } else {
        ["0", "0"]
    };
    let clock = LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &interval(clock_offset)?,
        &mut ledger()?,
    )?;
    let samples = if per_sample {
        [["199/100", "201/100"], ["399/100", "401/100"]]
    } else {
        [["2", "2"], ["4", "4"]]
    };
    let samples = samples
        .into_iter()
        .map(interval)
        .collect::<TestResult<Vec<_>>>()?;
    Ok(LcSharedClockPhase::new(
        clock,
        samples,
        &interval(phase_offset)?,
        &mut ledger()?,
    )?)
}

/// Exact coherent finite supports preserve signed phase and clock error coordinates.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Every retained systematic variable is explicit in the original physical witness parameters.
pub(super) fn parameters(
    rate: &str,
    initial: &str,
    clock_offset: &str,
    phase_offset: &str,
    gauge: &str,
) -> TestResult<LcSharedClockPhaseParameters> {
    Ok(LcSharedClockPhaseParameters {
        physical: LcClockPhaseParameters {
            rate: number(rate)?,
            initial_phase: number(initial)?,
            gauge: number(gauge)?,
            offset: number(clock_offset)?,
            ratio: number("1")?,
            voltage: number("1")?,
        },
        offset: number(phase_offset)?,
    })
}

/// One unit of remaining Work distinguishes success from a typed exhausted-Work outcome.
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
        budget.work(17, nrz_cas::Location::new("combined clock phase prefix"))?;
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

/// A valid original shared clock still refuses separate offsets for the two measured phases.
pub(super) fn independent_phase_control(source: &LcSharedClockPhase) -> TestResult {
    let cover = source
        .clock()
        .pairs_cover(super::cycles(), &super::pi()?, &mut ledger()?)?;
    let parameters = parameters("19/10", "0", "0", "0", "0")?;
    let point = cover.witness_with_phase(&parameters.physical, &mut ledger()?)?;
    source.clock().check_witness(
        point.original().values().clone(),
        number("0")?,
        &mut ledger()?,
    )?;
    for offset in ["-1/10", "-1/5"] {
        assert!(
            source
                .check_witness(
                    point.original().values().clone(),
                    number("0")?,
                    number(offset)?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    independent_clock_control(source, point.original().values().clone())
}

/// A valid common phase offset cannot replace the separately retained shared clock equation.
fn independent_clock_control(
    source: &LcSharedClockPhase,
    mut values: nrz_bindings::LcValues,
) -> TestResult {
    values.rate = number("21/10")?.into();
    values.inductance = number("10/21")?.into();
    values.capacitance = number("10/21")?.into();
    values.energy = number("5/21")?.into();
    for (observation, (time, phase)) in values
        .observations
        .iter_mut()
        .zip([("20/21", "2"), ("40/21", "4")])
    {
        observation.time = number(time)?.into();
        observation.phase = Some(number(phase)?.into());
    }
    let timing = nrz_bindings::LcSampleTimes::new(
        source.relation().clone(),
        source.clock().nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    let phase = nrz_bindings::LcSharedPhase::new(
        timing,
        source.samples().to_vec(),
        &interval(["-1/5", "1/5"])?,
        &mut ledger()?,
    )?;
    phase.check_witness(values.clone(), number("0")?, &mut ledger()?)?;
    for offset in ["-1/21", "-2/21"] {
        assert!(
            source
                .check_witness(
                    values.clone(),
                    number(offset)?,
                    number("0")?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    Ok(())
}

/// Both shared errors and each error alone retain the same original physical input inventory.
pub(super) fn separate_layer_controls() -> TestResult {
    for (clock_shared, phase_shared, expected) in [
        (true, true, ["-1/5", "1/5"]),
        (true, false, ["0", "0"]),
        (false, true, ["0", "0"]),
    ] {
        let source = question([clock_shared, phase_shared, true, false])?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
        assert_eq!(
            cover.offset_at_phase(&number("2")?, &number("0")?, &mut ledger()?)?,
            [number(expected[0])?, number(expected[1])?]
        );
        assert!(cover.phase().shares_allocation(&source));
        assert!(cover.relation().shares_allocation(source.relation()));
        assert_eq!(cover.cycles(), cycles());
        let point = cover.feasibility_witness();
        assert!(point.phase().shares_allocation(&source));
        assert!(point.original().clock().shares_allocation(source.clock()));
        assert!(
            point
                .original()
                .original()
                .relation()
                .shares_allocation(source.relation())
        );
        source.check_witness(
            point.original().original().values().clone(),
            point.original().offset().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Narrow original total timestamps retain both original time-error layers and physical initial phase.

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    Budget, LcClockPhaseParameters, LcInput, LcInterval, LcPhaseGauge, LcQuantity as Q, LcRelation,
    LcRelationError, LcSharedTiming, Limits, LimitsOverride,
};

/// Original time boxes narrow both declared images while both independent error supports stay unchanged.
fn question(known: bool, extra: Vec<LcInput>) -> TestResult<LcSharedTiming> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    for (index, (time, phase)) in [(["99/100", "101/100"], "2"), (["199/100", "201/100"], "4")]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    inputs.extend(extra);
    let relation = LcRelation::new(
        "narrow original two-layer timestamps".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSharedTiming::new(
        relation,
        vec![number("1")?, number("2")?],
        &[interval(["-1/100", "1/100"])?, interval(["-1/50", "1/50"])?],
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?)
}

/// Original time intersections and one shared offset give exact rational rate and offset fibres.
/// ´test:integration:lc-relation-narrower-both-timing-originals´
#[test]
fn lc_relation_narrower_both_timing_originals() -> TestResult {
    let timing = question(true, Vec::new())?;
    assert_eq!(timing.offset(), &[number("-1/10")?, number("1/10")?]);
    assert_eq!(
        timing.offset_support(),
        &[number("-1/50")?, number("1/50")?]
    );
    assert_eq!(
        timing.time_boxes(),
        &[
            [number("99/100")?, number("101/100")?],
            [number("199/100")?, number("201/100")?]
        ]
    );
    let cover = timing.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(cover.timing().shares_allocation(&timing));
    assert!(cover.relation().shares_allocation(timing.relation()));
    assert_eq!(cover.rate(), &[number("400/201")?, number("400/199")?]);
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::Fixed);
    for (rate, offsets, times) in [
        ("400/201", ["-1/200", "3/200"], ["201/200", "201/100"]),
        ("2", ["-1/100", "1/100"], ["1", "2"]),
        ("400/199", ["-3/200", "1/200"], ["199/200", "199/100"]),
    ] {
        assert_eq!(
            cover.offset_at_phase(&number(rate)?, &number("0")?, &mut ledger()?)?,
            [number(offsets[0])?, number(offsets[1])?]
        );
        for offset in offsets {
            let point =
                cover.witness_with_phase(&parameters(rate, "0", offset, "0")?, &mut ledger()?)?;
            for (observation, time) in point.original().values().observations.iter().zip(times) {
                assert_eq!(observation.time, number(time)?);
            }
            timing.check_witness(
                point.original().values().clone(),
                point.offset().clone(),
                &mut ledger()?,
            )?;
        }
    }
    Ok(())
}

/// Unknown physical phase and all common gauges retain conditional original time intersections.
/// ´test:integration:lc-relation-narrower-both-timing-phase-gauges´
#[test]
fn lc_relation_narrower_both_timing_phase_gauges() -> TestResult {
    let timing = question(false, Vec::new())?;
    let cover = timing.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("100/51")?, number("100/49")?]);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/50")?, number("1/50")?]
    );
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    for (alpha, offsets) in [("1/50", ["-1/50", "0"]), ("-1/50", ["0", "1/50"])] {
        assert_eq!(
            cover.offset_at_phase(&number("2")?, &number(alpha)?, &mut ledger()?)?,
            [number(offsets[0])?, number(offsets[1])?]
        );
        for offset in offsets {
            for gauge in ["-2", "0", "3"] {
                let point = cover
                    .witness_with_phase(&parameters("2", alpha, offset, gauge)?, &mut ledger()?)?;
                timing.check_witness(
                    point.original().values().clone(),
                    point.offset().clone(),
                    &mut ledger()?,
                )?;
            }
        }
    }
    let point = cover.witness_with_phase(&parameters("2", "1/50", "0", "0")?, &mut ledger()?)?;
    timing
        .relation()
        .check_witness(point.original().values().clone(), &mut ledger()?)?;
    assert!(
        timing
            .check_witness(
                point.original().values().clone(),
                number("1/50")?,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(
        cover
            .witness_with_phase(&parameters("2", "1/50", "1/50", "0")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Repeated originals and disjoint common-offset images cannot be replaced by separate sample boxes.
/// ´test:integration:lc-relation-narrower-both-timing-refusals-work´
#[test]
fn lc_relation_narrower_both_timing_refusals_work() -> TestResult {
    const FIRST: usize = 0;
    let timing = question(false, vec![input(Q::Time(FIRST), ["1", "101/100"])?])?;
    assert_eq!(timing.time_boxes()[0], [number("1")?, number("101/100")?]);
    assert!(question(false, vec![input(Q::Time(FIRST), ["0", "0"])?]).is_err());
    disjoint_offsets()?;
    let errors = [interval(["-1/100", "1/100"])?, interval(["-1/50", "1/50"])?];
    let offset = interval(["-1/10", "1/10"])?;
    boundary(|budget| {
        LcSharedTiming::new(
            timing.relation().clone(),
            timing.nominal_times().to_vec(),
            &errors,
            &offset,
            budget,
        )
    })?;
    let pi = pi()?;
    boundary(|budget| timing.pairs_cover(cycles(), &pi, budget))?;
    let cover = timing.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let parameters = parameters("2", "0", "0", "0")?;
    boundary(|budget| cover.witness_with_phase(&parameters, budget))?;
    boundary(|budget| cover.phase_at_rate(&parameters.rate, budget))?;
    boundary(|budget| cover.offset_at_rate(&parameters.rate, budget))?;
    boundary(|budget| cover.offset_at_phase(&parameters.rate, &parameters.initial_phase, budget))?;
    boundary(|budget| cover.forward_horizon(budget))?;
    Ok(())
}

/// Individually feasible exact timestamps require disjoint common-offset intervals.
fn disjoint_offsets() -> TestResult {
    const FIRST: usize = 0;
    const SECOND: usize = 1;
    let original = question(false, Vec::new())?;
    let mut inputs = original.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Time(_)));
    inputs.push(input(Q::Time(FIRST), ["20/21", "20/21"])?);
    inputs.push(input(Q::Time(SECOND), ["40/21", "40/21"])?);
    let relation = LcRelation::new(
        original.relation().epoch().into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let errors = [interval(["-1/100", "1/100"])?, interval(["-1/50", "1/50"])?];
    let offset = interval(["-1/10", "1/10"])?;
    assert!(
        LcSharedTiming::new(
            relation,
            original.nominal_times().to_vec(),
            &errors,
            &offset,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Every original systematic and physical coordinate is chosen explicitly.
fn parameters(
    rate: &str,
    alpha: &str,
    offset: &str,
    gauge: &str,
) -> TestResult<LcClockPhaseParameters> {
    Ok(LcClockPhaseParameters {
        rate: number(rate)?,
        initial_phase: number(alpha)?,
        offset: number(offset)?,
        gauge: number(gauge)?,
        ratio: number("1")?,
        voltage: number("1")?,
    })
}

/// Finite signed rational supports keep the original coherent coordinates.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// The consumed caller prefix survives both exact success and the adjacent typed Work refusal.
fn boundary<T>(operation: impl Fn(&mut Budget) -> Result<T, LcRelationError>) -> TestResult {
    let mut measured = ledger()?;
    operation(&mut measured)?;
    let work = measured.usage().work;
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?)?;
        budget.work(17, nrz_cas::Location::new("narrow both timing prefix"))?;
        let outcome = operation(&mut budget);
        assert_eq!(outcome.is_ok(), success);
        if !success {
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

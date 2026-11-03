// SPDX-License-Identifier: AGPL-3.0-only
//! Shared-clock reconstruction retains original physical initial phase and all common gauges.

mod support;

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{ExactConstant, LcPairParameters, LcPhaseGauge, LcQuantity as Q, LcSampleTimes};
use support::{clock, parameters, work_boundary};

/// Unknown physical initial phase and clock offset retain their exact conditional correlation.
/// ´test:integration:lc-relation-clock-unknown-initial-phase´
#[test]
fn lc_relation_clock_unknown_initial_phase() -> TestResult {
    let clock = clock(None, ["-1/10", "1/10"], Vec::new())?;
    let cover = clock.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/5")?, number("1/5")?]
    );
    assert_eq!(
        cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/10")?, number("1/10")?]
    );
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_eq!(cover.cycles(), cycles());
    assert!(cover.clock().shares_allocation(&clock));
    assert!(cover.relation().shares_allocation(clock.relation()));
    for (phase, offset) in [("-1/5", "1/10"), ("0", "0"), ("1/5", "-1/10")] {
        assert_eq!(
            cover.offset_at_phase(&number("2")?, &number(phase)?, &mut ledger()?)?,
            [number(offset)?, number(offset)?]
        );
        let point =
            cover.witness_with_phase(&parameters("2", phase, offset, "0")?, &mut ledger()?)?;
        assert!(point.clock().shares_allocation(&clock));
        assert!(
            point
                .original()
                .relation()
                .shares_allocation(clock.relation())
        );
        assert_eq!(
            point.original().coordinate(Q::InitialPhase),
            Some(&number(phase)?.into())
        );
        clock.check_witness(
            point.original().values().clone(),
            number(offset)?,
            &mut ledger()?,
        )?;
    }
    // Both coordinates are marginal endpoints, but alpha+a*offset is not the observed beta.
    assert!(
        cover
            .witness_with_phase(&parameters("2", "1/5", "1/10", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .offset_at_phase(&number("2")?, &number("1/4")?, &mut ledger()?)
            .is_err()
    );
    assert!(cover.bounds(Q::InitialPhase, &mut ledger()?).is_err());
    Ok(())
}

/// Removing the shared clock permits rates and original tuples excluded by the joint model.
/// ´test:integration:lc-relation-clock-initial-marginal-control´
#[test]
fn lc_relation_clock_initial_marginal_control() -> TestResult {
    let clock = clock(None, ["-1/10", "1/10"], Vec::new())?;
    let shared = clock.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let independent = LcSampleTimes::new(
        clock.relation().clone(),
        clock.nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    let marginal = independent.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(marginal.rate(), &[number("5/3")?, number("5/2")?]);
    assert_eq!(shared.rate(), &[number("2")?, number("2")?]);
    let point = marginal.witness(
        &LcPairParameters {
            rate: number("5/3")?,
            initial_phase: number("1/2")?,
            gauge: number("0")?,
            ratio: number("1")?,
            voltage: number("1")?,
        },
        &mut ledger()?,
    )?;
    clock
        .relation()
        .check_witness(point.values().clone(), &mut ledger()?)?;
    assert!(
        clock
            .check_witness(point.values().clone(), number("-1/10")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        clock
            .check_witness(point.values().clone(), number("1/10")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Every common integer lift and signed offset domain restores the same original equations.
/// ´test:integration:lc-relation-clock-initial-gauges´
#[test]
fn lc_relation_clock_initial_gauges() -> TestResult {
    let turn = ExactConstant::named(nrz_cas::syntax::Constant::Pi, &mut ledger()?)?
        .mul(&number("2")?.into(), &mut ledger()?)?;
    for (offsets, points) in [
        (["-1/10", "1/10"], [("1/5", "-1/10"), ("-1/5", "1/10")]),
        (["1/10", "1/5"], [("-1/5", "1/10"), ("-2/5", "1/5")]),
        (["-1/5", "-1/10"], [("2/5", "-1/5"), ("1/5", "-1/10")]),
    ] {
        let clock = clock(None, offsets, Vec::new())?;
        let cover = clock.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        for (phase, offset) in points {
            for gauge in ["-2", "0", "3"] {
                let point = cover
                    .witness_with_phase(&parameters("2", phase, offset, gauge)?, &mut ledger()?)?;
                let expected = ExactConstant::from(number(phase)?).add(
                    &turn.mul(&number(gauge)?.into(), &mut ledger()?)?,
                    &mut ledger()?,
                )?;
                assert!(
                    point
                        .original()
                        .coordinate(Q::InitialPhase)
                        .ok_or("physical initial phase")?
                        .structurally_equal(&expected, &mut ledger()?)?
                );
                for observation in &point.original().values().observations {
                    assert_eq!(observation.cycle.as_ref(), Some(&number(gauge)?));
                }
                clock.check_witness(
                    point.original().values().clone(),
                    number(offset)?,
                    &mut ledger()?,
                )?;
            }
        }
    }
    Ok(())
}

/// Repeated bounded physical phase inputs constrain the clock jointly and fixed nonzero phase survives.
/// ´test:integration:lc-relation-clock-bounded-initial-phase´
#[test]
fn lc_relation_clock_bounded_initial_phase() -> TestResult {
    let clock = clock(
        Some(["0", "1/5"]),
        ["-1/10", "1/10"],
        vec![input(Q::InitialPhase, ["1/20", "3/20"])?],
    )?;
    let cover = clock.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        clock.initial_phase_support(),
        Some(&[number("1/20")?, number("3/20")?])
    );
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::Fixed);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("1/20")?, number("3/20")?]
    );
    assert_eq!(
        cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-3/40")?, number("-1/40")?]
    );
    for (phase, offset) in [("1/20", "-1/40"), ("1/10", "-1/20"), ("3/20", "-3/40")] {
        let point =
            cover.witness_with_phase(&parameters("2", phase, offset, "0")?, &mut ledger()?)?;
        clock.check_witness(
            point.original().values().clone(),
            number(offset)?,
            &mut ledger()?,
        )?;
    }
    assert!(
        cover
            .witness_with_phase(&parameters("2", "0", "0", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness_with_phase(&parameters("2", "1/10", "-1/20", "1")?, &mut ledger()?)
            .is_err()
    );
    support::fixed_nonzero()
}

/// Complete cycle evidence, original timing restrictions and cumulative Work remain explicit.
/// ´test:integration:lc-relation-clock-initial-refusals-work´
#[test]
fn lc_relation_clock_initial_refusals_work() -> TestResult {
    let clock = clock(None, ["-1/10", "1/10"], Vec::new())?;
    let pi = pi()?;
    assert!(
        clock
            .pairs_cover(vec![number("0")?, number("1")?], &pi, &mut ledger()?)
            .is_err()
    );
    let mut measured = ledger()?;
    let cover = clock.pairs_cover(cycles(), &pi, &mut measured)?;
    assert_eq!(
        cover.usage()[1].work - cover.usage()[0].work,
        measured.usage().work
    );
    work_boundary(|budget| clock.pairs_cover(cycles(), &pi, budget))?;
    let point = parameters("2", "1/5", "-1/10", "0")?;
    work_boundary(|budget| cover.witness_with_phase(&point, budget))?;
    work_boundary(|budget| cover.phase_at_rate(&point.rate, budget))?;
    work_boundary(|budget| cover.offset_at_phase(&point.rate, &point.initial_phase, budget))?;
    work_boundary(|budget| cover.offset_at_rate(&point.rate, budget))?;
    assert!(
        cover
            .witness_with_phase(&parameters("2", "-2/5", "1/5", "0")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Original timing fixtures, exact physical extremizers and independent numerical pi oracles.

use super::super::super::time::oracle::Pi;
use super::{TestResult, input, ledger, number};
use nrz_bindings::readings::algebra::Rational;
use nrz_bindings::{
    Budget, LcClockCover, LcClockPhaseParameters, LcHorizon, LcHorizonValue, LcInterval,
    LcQuantity as Q, LcRelation, LcRelationError, LcSampleTimes, LcSharedClock, LcSharedTiming,
    LcTimingCover, Limits, LimitsOverride,
};
use nrz_matrix::LcHorizonProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};
use std::cmp::Ordering;

/// Original time-box intersection retains the chosen first timestamp and a common later reference.
pub(super) fn timing(known: bool, first: [&str; 2]) -> TestResult<LcSampleTimes> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    for (index, (time, phase)) in [(first, "2"), (["9/5", "23/10"], "4")]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    let relation = LcRelation::new("timing horizon epoch".into(), 2, inputs, &mut ledger()?)?;
    Ok(LcSampleTimes::new(
        relation,
        vec![number("1")?, number("2")?],
        &mut ledger()?,
    )?)
}

/// One original physical question retains a clock offset, with known or unknown physical phase.
pub(super) fn clock(known: bool, uncertain: bool) -> TestResult<LcSharedClock> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    let phases = if uncertain {
        [["199/100", "201/100"], ["199/50", "201/50"]]
    } else {
        [["2", "2"], ["4", "4"]]
    };
    for (index, (time, phase)) in [["9/10", "11/10"], ["19/10", "21/10"]]
        .into_iter()
        .zip(phases)
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    let relation = LcRelation::new(
        "clock actual-time horizon epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?)
}

/// Separate sample errors and one shared offset reproduce the accepted joint timing fixture.
pub(super) fn both_timing() -> TestResult<LcSharedTiming> {
    let mut inputs = vec![
        input(Q::Rate, ["1", "3"])?,
        input(Q::InitialPhase, ["0", "0"])?,
    ];
    for (index, (time, phase)) in [(["89/100", "111/100"], "2"), (["47/25", "53/25"], "4")]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    let relation = LcRelation::new(
        "both timing horizon epoch".into(),
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

/// Original complete association uses the two zero representative lifts.
pub(super) fn cycles() -> Vec<Rational> {
    vec![Rational::zero(), Rational::zero()]
}

/// Signed coherent finite supports keep time and offset coordinates distinct.
fn interval(bounds: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(bounds[0])?.into()),
        Some(number(bounds[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Independent affine-pi coefficients specify the exact threshold.
pub(super) fn finite(horizon: &LcHorizon, offset: &str, coefficient: &str) -> TestResult {
    let LcHorizonValue::Finite(endpoint) = horizon.value() else {
        return Err("finite timing horizon".into());
    };
    assert_eq!(
        endpoint.coefficients(),
        (&number(offset)?, &number(coefficient)?)
    );
    Ok(())
}

/// Both numerical formats retain original source identity and independent Machin containment evidence.
pub(super) fn render<P: Scalar + 'static>(
    fixtures: &[(LcHorizon, &str, &str)],
    oracle: &Pi,
) -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = super::super::super::super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
    for (horizon, coefficient, offset) in fixtures {
        let native = LcHorizonProjection::new(
            horizon.clone(),
            pi.clone(),
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert!(
            native
                .exact()
                .relation()
                .shares_allocation(horizon.relation())
        );
        assert!(native.pi().same_region(&pi));
        assert_eq!(native.exact().coordinate(), horizon.coordinate());
        match horizon.value() {
            LcHorizonValue::Finite(_) => oracle.affine(
                native
                    .enclosure()
                    .ok_or("finite timing horizon enclosure")?,
                &number(coefficient)?,
                &number(offset)?,
            )?,
            LcHorizonValue::Infinite => assert!(native.enclosure().is_none()),
            LcHorizonValue::Undecided => {
                return Err("fixture horizon unexpectedly undecided".into());
            }
        }
    }
    Ok(())
}

/// The physical clock offset is recovered conditionally at each actual-time extremizer.
pub(super) fn clock_extrema(cover: &LcClockCover, horizon: &LcHorizon) -> TestResult {
    for point in horizon.extremal_parameters() {
        let offset = cover.offset_at_phase(&point[1], &point[0], &mut ledger()?)?[0].clone();
        let parameters = LcClockPhaseParameters {
            rate: point[1].clone(),
            initial_phase: point[0].clone(),
            gauge: number("0")?,
            offset,
            ratio: number("1")?,
            voltage: number("1")?,
        };
        let witness = cover.witness_with_phase(&parameters, &mut ledger()?)?;
        assert!(
            witness
                .original()
                .relation()
                .shares_allocation(horizon.relation())
        );
        cover.clock().check_witness(
            witness.original().values().clone(),
            witness.offset().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Future phases at endpoints and midpoint remain within the original-domain extrema.
pub(super) fn timing_future_extrema(cover: &LcTimingCover, horizon: &LcHorizon) -> TestResult {
    let middle = cover.rate()[0]
        .add(&cover.rate()[1], &mut ledger()?)?
        .div(&number("2")?, &mut ledger()?)?;
    for time in ["23/10", "3", "10"] {
        let time = number(time)?;
        let extrema = horizon.extremal_parameters();
        let lower =
            extrema[0][0].add(&extrema[0][1].mul(&time, &mut ledger()?)?, &mut ledger()?)?;
        let upper =
            extrema[1][0].add(&extrema[1][1].mul(&time, &mut ledger()?)?, &mut ledger()?)?;
        for rate in [&cover.rate()[0], &middle, &cover.rate()[1]] {
            for phase in cover.phase_at_rate(rate, &mut ledger()?)? {
                let future = phase.add(&rate.mul(&time, &mut ledger()?)?, &mut ledger()?)?;
                assert_ne!(future.compare(&lower, &mut ledger()?)?, Ordering::Less);
                assert_ne!(future.compare(&upper, &mut ledger()?)?, Ordering::Greater);
            }
        }
    }
    Ok(())
}

/// Adjacent cumulative budgets distinguish success from explicit Work exhaustion.
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
        budget.work(
            17,
            nrz_bindings::readings::algebra::Location::new("timing horizon prefix"),
        )?;
        let outcome = operation(&mut budget);
        assert_eq!(outcome.is_ok(), succeeds);
        if !succeeds {
            assert!(matches!(
                outcome,
                Err(LcRelationError::Algebra(
                    nrz_bindings::readings::algebra::Fault {
                        kind: nrz_bindings::readings::algebra::FaultKind::Exhausted {
                            budget: nrz_bindings::readings::algebra::limits::BudgetKind::Work,
                            ..
                        },
                        ..
                    }
                ))
            ));
        }
    }
    Ok(())
}

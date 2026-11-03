// SPDX-License-Identifier: AGPL-3.0-only
//! Shared-clock refinement horizons retain original source evidence and independent enclosures in both formats.

use super::support::{both_timing, clock, cycles, finite, render};
use super::{Pi, TestResult, input, ledger, number, pi_certificate};
use nrz_bindings::{
    LcClockRefinementState, LcInterval, LcQuantity as Q, LcRelation, LcSharedClock, LcSharedTiming,
};

/// Both native formats enclose no-earlier actual and reported horizons of the complete refined clock family.
/// ´test:integration:lc-relation-clock-refinement-horizons´
#[test]
fn lc_relation_clock_refinement_horizons() -> TestResult {
    let pi = pi_certificate()?;
    let mut fixtures = Vec::new();
    for known in [true, false] {
        let old = clock(known, true)?;
        let refined = narrower_clock(&old)?;
        let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
        let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        for reported in [false, true] {
            let horizon = if reported {
                previous.forward_reported_horizon(&mut ledger()?)?
            } else {
                previous.forward_horizon(&mut ledger()?)?
            };
            let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
            let LcClockRefinementState::Refined(success) = attempt.state() else {
                return Err("complete refined clock".into());
            };
            let offset = if reported {
                "4/3"
            } else if known {
                "0"
            } else {
                "-16/3"
            };
            finite(success.horizon(), offset, "200/3")?;
            assert_eq!(success.horizon().coordinate(), horizon.coordinate());
            assert_eq!(
                success.horizon().mathematical().reference(),
                horizon.mathematical().reference()
            );
            assert_eq!(
                success.product_inclusion(),
                &[
                    [number("10000/41209")?, number("10000/38809")?],
                    [number("40000/162409")?, number("40000/157609")?]
                ]
            );
            assert!(
                !success
                    .finite_horizon_margin()
                    .ok_or("finite refinement margin")?
                    .is_negative()
            );
            assert!(
                success
                    .clock_cover()
                    .ok_or("original refined clock")?
                    .clock()
                    .shares_allocation(&refined)
            );
            fixtures.push((success.horizon().clone(), "200/3", offset));
        }
    }
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// Narrow original timestamps preserve both declared error layers and reported-coordinate correlation.
/// ´test:integration:lc-relation-both-timing-refinement-horizons´
#[test]
fn lc_relation_both_timing_refinement_horizons() -> TestResult {
    let pi = pi_certificate()?;
    let mut fixtures = Vec::new();
    for known in [true, false] {
        let old = original_timing(known)?;
        let refined = narrower_timing(&old)?;
        let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
        let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        for reported in [false, true] {
            let horizon = if reported {
                previous.forward_reported_horizon(&mut ledger()?)?
            } else {
                previous.forward_horizon(&mut ledger()?)?
            };
            let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
            let LcClockRefinementState::Refined(success) = attempt.state() else {
                return Err("complete refined two-layer timing".into());
            };
            let offset = if reported {
                if known { "-1" } else { "1" }
            } else if known {
                "0"
            } else {
                "3/2"
            };
            let coefficient = if known { "39999/400" } else { "2499/100" };
            finite(success.horizon(), offset, coefficient)?;
            assert_eq!(success.horizon().coordinate(), horizon.coordinate());
            assert_eq!(
                success.horizon().mathematical().reference(),
                horizon.mathematical().reference()
            );
            assert!(
                !success
                    .finite_horizon_margin()
                    .ok_or("finite timing margin")?
                    .is_negative()
            );
            assert!(
                success
                    .shared_timing_cover()
                    .ok_or("original refined timing")?
                    .timing()
                    .shares_allocation(&refined)
            );
            fixtures.push((success.horizon().clone(), coefficient, offset));
        }
    }
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// Original identities and physical timestamps remain unchanged while phase strips and declared offset halve.
fn narrower_clock(old: &LcSharedClock) -> TestResult<LcSharedClock> {
    let mut inputs = old.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Phase(_)));
    for (index, phase) in [["399/200", "401/200"], ["399/100", "401/100"]]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Phase(index), phase)?);
    }
    let relation = LcRelation::new(old.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    Ok(LcSharedClock::new(
        relation,
        old.nominal_times().to_vec(),
        &interval(["-1/20", "1/20"])?,
        &mut ledger()?,
    )?)
}

/// The original both-layer fixture allows either known or free physical initial phase.
fn original_timing(known: bool) -> TestResult<LcSharedTiming> {
    let old = both_timing()?;
    let mut inputs = old.relation().inputs().to_vec();
    if !known {
        inputs.retain(|value| value.quantity() != Q::InitialPhase);
    }
    timing(&old, inputs, ["-1/10", "1/10"])
}

/// Only original timestamps and the one declared shared offset narrow.
fn narrower_timing(old: &LcSharedTiming) -> TestResult<LcSharedTiming> {
    let mut inputs = old.relation().inputs().to_vec();
    for (index, time) in [["99/100", "101/100"], ["199/100", "201/100"]]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
    }
    timing(old, inputs, ["-1/20", "1/20"])
}

/// Rebuild one complete original measurement question with identical independent errors.
fn timing(
    old: &LcSharedTiming,
    inputs: Vec<nrz_bindings::LcInput>,
    offset: [&str; 2],
) -> TestResult<LcSharedTiming> {
    let relation = LcRelation::new(old.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    Ok(LcSharedTiming::new(
        relation,
        old.nominal_times().to_vec(),
        &[interval(["-1/100", "1/100"])?, interval(["-1/50", "1/50"])?],
        &interval(offset)?,
        &mut ledger()?,
    )?)
}

/// Exact signed clock and independent error supports preserve coherent time units.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

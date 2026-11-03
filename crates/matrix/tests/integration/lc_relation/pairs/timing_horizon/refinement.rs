// SPDX-License-Identifier: AGPL-3.0-only
//! Shrinking one original time box preserves inclusion on a fixed common future domain.

use super::support::{cycles, finite, timing, work_boundary};
use super::{TestResult, ledger, number, pi_certificate};
use nrz_bindings::{LcHorizon, LcHorizonValue, LcPairParameters, LcTimingCover};
use std::cmp::Ordering;

/// Original input inclusion and exact time elimination retain nested fibres and no-earlier horizons.
/// ´test:integration:lc-relation-timestamp-refinement-horizon´
#[test]
fn lc_relation_timestamp_refinement_horizon() -> TestResult {
    let reference = number("23/10")?;
    let pi = pi_certificate()?;
    for known in [true, false] {
        let mut original = timing(known, ["9/10", "11/10"])?;
        let mut previous = original.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        let mut old_horizon = previous.forward_horizon_from(&reference, &mut ledger()?)?;
        for (bounds, expected) in [
            (
                ["19/20", "21/20"],
                Some(if known {
                    ["0", "399/40"]
                } else {
                    ["47/40", "27/16"]
                }),
            ),
            (
                ["99/100", "101/100"],
                Some(if known {
                    ["0", "9999/200"]
                } else {
                    ["541/520", "10349/5200"]
                }),
            ),
            (["1", "1"], if known { None } else { Some(["1", "52/25"]) }),
        ] {
            let next = timing(known, bounds)?;
            let inclusion = original
                .relation()
                .refinement_inputs(next.relation(), &mut ledger()?)?;
            assert!(inclusion.previous().shares_allocation(original.relation()));
            assert!(inclusion.refined().shares_allocation(next.relation()));
            let cover = next.pairs_cover(cycles(), &pi, &mut ledger()?)?;
            included(&previous, &cover)?;
            let horizon = cover.forward_horizon_from(&reference, &mut ledger()?)?;
            assert_eq!(
                horizon.mathematical().reference(),
                old_horizon.mathematical().reference()
            );
            match expected {
                Some([offset, coefficient]) => finite(&horizon, offset, coefficient)?,
                None => assert!(matches!(horizon.value(), LcHorizonValue::Infinite)),
            }
            no_earlier(&old_horizon, &horizon)?;
            original = next;
            previous = cover;
            old_horizon = horizon;
        }
    }
    Ok(())
}

/// A widened timestamp refuses original input inclusion; its exhausted check preserves both owners.
/// ´test:integration:lc-relation-timestamp-refinement-refusals-work´
#[test]
fn lc_relation_timestamp_refinement_refusals_work() -> TestResult {
    let previous = timing(false, ["9/10", "11/10"])?;
    let next = timing(false, ["19/20", "21/20"])?;
    assert!(
        next.relation()
            .refinement_inputs(previous.relation(), &mut ledger()?)
            .is_err()
    );
    work_boundary(|budget| {
        previous
            .relation()
            .refinement_inputs(next.relation(), budget)
    })?;
    assert_eq!(
        previous.time_boxes()[0],
        [number("9/10")?, number("11/10")?]
    );
    assert_eq!(next.time_boxes()[0], [number("19/20")?, number("21/20")?]);
    Ok(())
}

/// Input inclusion proves the whole feasible-set implication; endpoint/midpoint witnesses audit restoration.
fn included(previous: &LcTimingCover, next: &LcTimingCover) -> TestResult {
    assert_eq!(next.phase_gauge(), previous.phase_gauge());
    assert_eq!(next.cycles(), previous.cycles());
    assert_ne!(
        next.rate()[0].compare(&previous.rate()[0], &mut ledger()?)?,
        Ordering::Less
    );
    assert_ne!(
        next.rate()[1].compare(&previous.rate()[1], &mut ledger()?)?,
        Ordering::Greater
    );
    let middle = next.rate()[0]
        .add(&next.rate()[1], &mut ledger()?)?
        .div(&number("2")?, &mut ledger()?)?;
    for rate in [&next.rate()[0], &middle, &next.rate()[1]] {
        let old = previous.phase_at_rate(rate, &mut ledger()?)?;
        let new = next.phase_at_rate(rate, &mut ledger()?)?;
        assert_ne!(new[0].compare(&old[0], &mut ledger()?)?, Ordering::Less);
        assert_ne!(new[1].compare(&old[1], &mut ledger()?)?, Ordering::Greater);
        for phase in new {
            let parameters = LcPairParameters {
                rate: rate.clone(),
                initial_phase: phase,
                gauge: number("0")?,
                ratio: number("1")?,
                voltage: number("1")?,
            };
            let witness = next.witness(&parameters, &mut ledger()?)?;
            previous
                .relation()
                .check_witness(witness.values().clone(), &mut ledger()?)?;
        }
    }
    Ok(())
}

/// Exact affine coefficients and retained pi bounds certify the ordering, including explicit infinity.
fn no_earlier(previous: &LcHorizon, next: &LcHorizon) -> TestResult {
    assert!(!matches!(previous.value(), LcHorizonValue::Undecided));
    match next.value() {
        LcHorizonValue::Infinite => Ok(()),
        LcHorizonValue::Finite(endpoint) => {
            let LcHorizonValue::Finite(old) = previous.value() else {
                return Err("finite refinement of infinite horizon".into());
            };
            let (offset, coefficient) = endpoint.coefficients();
            let (old_offset, old_coefficient) = old.coefficients();
            let offset = offset.sub(old_offset, &mut ledger()?)?;
            let coefficient = coefficient.sub(old_coefficient, &mut ledger()?)?;
            let pi = previous.mathematical().pi_bounds().contract();
            let bound = if coefficient.is_negative() {
                &pi.upper
            } else {
                &pi.lower
            };
            let lower_difference =
                offset.add(&coefficient.mul(bound, &mut ledger()?)?, &mut ledger()?)?;
            assert!(!lower_difference.is_negative());
            Ok(())
        }
        LcHorizonValue::Undecided => Err("undecided refined horizon".into()),
    }
}

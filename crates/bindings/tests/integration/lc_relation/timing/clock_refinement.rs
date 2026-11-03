// SPDX-License-Identifier: AGPL-3.0-only
//! Whole-question clock refinement preserves physical witnesses, both coordinate contracts and old evidence.

mod refusals;
mod support;
mod work;

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    LcClockCover, LcClockRefinementState, LcHorizonCoordinate, LcHorizonValue, LcSharedTimingCover,
};
use std::cmp::Ordering;
use support::{clock, finite, parameters, timing};

/// Narrow original phase strips and clock offsets give nested original families and no-earlier horizons.
/// ´test:integration:lc-relation-shared-clock-refinement´
#[test]
fn lc_relation_shared_clock_refinement() -> TestResult {
    for known in [true, false] {
        for reported in [false, true] {
            clock_sequence(known, reported)?;
        }
    }
    Ok(())
}

/// Each coordinate uses the same original future reference through both refinements.
fn clock_sequence(known: bool, reported: bool) -> TestResult {
    let pi = pi()?;
    let mut original = clock(known, "1/100", ["-1/10", "1/10"])?;
    let mut previous = original.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let mut horizon = if reported {
        previous.forward_reported_horizon(&mut ledger()?)?
    } else {
        previous.forward_horizon(&mut ledger()?)?
    };
    for (width, offsets) in [("1/200", ["-1/20", "1/20"]), ("0", ["0", "0"])] {
        let refined = clock(known, width, offsets)?;
        let inclusion = original.refinement_inputs(&refined, &mut ledger()?)?;
        let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
        let LcClockRefinementState::Refined(success) = attempt.state() else {
            return Err(format!("clock refinement {:?}", attempt.state()).into());
        };
        let next = success.clock_cover().ok_or("single-clock success")?;
        included_clock(&previous, next)?;
        assert!(
            attempt
                .previous_clock()
                .ok_or("previous clock")?
                .shares_allocation(&previous)
        );
        assert!(
            attempt
                .inclusion()
                .refined_clock()
                .ok_or("new question")?
                .shares_allocation(&refined)
        );
        assert_eq!(
            success.horizon().coordinate(),
            if reported {
                LcHorizonCoordinate::ReportedClock
            } else {
                LcHorizonCoordinate::ActualTime
            }
        );
        assert_eq!(
            success.horizon().mathematical().reference(),
            horizon.mathematical().reference()
        );
        if width == "0" {
            assert!(matches!(
                success.horizon().value(),
                LcHorizonValue::Infinite
            ));
            assert!(success.finite_horizon_margin().is_none());
        } else {
            finite(
                success.horizon(),
                if reported {
                    "4/3"
                } else if known {
                    "0"
                } else {
                    "-16/3"
                },
                "200/3",
            )?;
            assert!(
                !success
                    .finite_horizon_margin()
                    .ok_or("finite monotonicity")?
                    .is_negative()
            );
        }
        original = refined;
        previous = next.clone();
        horizon = success.horizon().clone();
    }
    Ok(())
}

/// Both original time-error layers narrow jointly, including the restricted physical timestamp path.
/// ´test:integration:lc-relation-both-timing-refinement´
#[test]
fn lc_relation_both_timing_refinement() -> TestResult {
    for known in [true, false] {
        for reported in [false, true] {
            timing_sequence(known, reported)?;
        }
    }
    Ok(())
}

/// Narrow original times first, then narrow the original independent sample errors.
fn timing_sequence(known: bool, reported: bool) -> TestResult {
    let pi = pi()?;
    let old = timing(known, false, false)?;
    let refined = timing(known, true, false)?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = if reported {
        previous.forward_reported_horizon(&mut ledger()?)?
    } else {
        previous.forward_horizon(&mut ledger()?)?
    };
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("both-time refinement {:?}", attempt.state()).into());
    };
    let next = success.shared_timing_cover().ok_or("both time layers")?;
    included_timing(&previous, next)?;
    assert!(
        attempt
            .previous_shared_timing()
            .ok_or("old timing")?
            .shares_allocation(&previous)
    );
    finite(
        success.horizon(),
        if reported {
            if known { "-1" } else { "1" }
        } else if known {
            "0"
        } else {
            "3/2"
        },
        if known { "39999/400" } else { "2499/100" },
    )?;
    let smaller = timing(known, true, true)?;
    let inclusion = refined.refinement_inputs(&smaller, &mut ledger()?)?;
    let again = next.refine(&inclusion, success.horizon(), cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(last) = again.state() else {
        return Err(format!("smaller independent errors {:?}", again.state()).into());
    };
    included_timing(next, last.shared_timing_cover().ok_or("smaller timing")?)?;
    Ok(())
}

/// Whole-question inclusion is audited at conditional original physical and clock endpoint fibres.
fn included_clock(previous: &LcClockCover, refined: &LcClockCover) -> TestResult {
    for rate in refined.rate() {
        let old = previous.phase_at_rate(rate, &mut ledger()?)?;
        let new = refined.phase_at_rate(rate, &mut ledger()?)?;
        included(&old, &new)?;
        for alpha in new {
            let old = previous.offset_at_phase(rate, &alpha, &mut ledger()?)?;
            let new = refined.offset_at_phase(rate, &alpha, &mut ledger()?)?;
            included(&old, &new)?;
            for offset in new {
                let point = refined.witness_with_phase(
                    &parameters(rate.clone(), alpha.clone(), offset),
                    &mut ledger()?,
                )?;
                previous.clock().check_witness(
                    point.original().values().clone(),
                    point.offset().clone(),
                    &mut ledger()?,
                )?;
            }
        }
    }
    Ok(())
}

/// Each refined conditional timestamp witness satisfies the old complete measurement question.
fn included_timing(previous: &LcSharedTimingCover, refined: &LcSharedTimingCover) -> TestResult {
    included(previous.rate(), refined.rate())?;
    for rate in refined.rate() {
        let old = previous.phase_at_rate(rate, &mut ledger()?)?;
        let new = refined.phase_at_rate(rate, &mut ledger()?)?;
        included(&old, &new)?;
        for alpha in new {
            let offsets = refined.offset_at_phase(rate, &alpha, &mut ledger()?)?;
            included(
                &previous.offset_at_phase(rate, &alpha, &mut ledger()?)?,
                &offsets,
            )?;
            for offset in offsets {
                let point = refined.witness_with_phase(
                    &parameters(rate.clone(), alpha.clone(), offset),
                    &mut ledger()?,
                )?;
                previous.timing().check_witness(
                    point.original().values().clone(),
                    point.offset().clone(),
                    &mut ledger()?,
                )?;
            }
        }
    }
    Ok(())
}

/// Exact rational interval inclusion checks both signed endpoints.
fn included(old: &[nrz_cas::Rational; 2], new: &[nrz_cas::Rational; 2]) -> TestResult {
    assert_ne!(new[0].compare(&old[0], &mut ledger()?)?, Ordering::Less);
    assert_ne!(new[1].compare(&old[1], &mut ledger()?)?, Ordering::Greater);
    Ok(())
}

/// Added original observations retain old identities and require one common future reference.
/// ´test:integration:lc-relation-clock-refinement-added-observation´
#[test]
fn lc_relation_clock_refinement_added_observation() -> TestResult {
    const ADDED: usize = 2;
    let old = clock(true, "1/100", ["-1/10", "1/10"])?;
    let base = clock(true, "1/200", ["-1/20", "1/20"])?;
    let mut inputs = base.relation().inputs().to_vec();
    inputs.push(input(nrz_bindings::LcQuantity::Time(ADDED), ["3", "3"])?);
    inputs.push(input(
        nrz_bindings::LcQuantity::Phase(ADDED),
        ["599/100", "601/100"],
    )?);
    let relation =
        nrz_bindings::LcRelation::new(old.relation().epoch().into(), 3, inputs, &mut ledger()?)?;
    let refined = nrz_bindings::LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?, number("3")?],
        &support::interval(["-1/20", "1/20"])?,
        &mut ledger()?,
    )?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let early = previous.forward_reported_horizon(&mut ledger()?)?;
    let horizon = previous.forward_reported_horizon_from(&number("4")?, &mut ledger()?)?;
    let mut proposed = cycles();
    proposed.push(number("0")?);
    let unresolved = previous.refine(&inclusion, &early, proposed.clone(), &mut ledger()?)?;
    assert!(matches!(
        unresolved.state(),
        LcClockRefinementState::Undecided(nrz_bindings::LcRelationError::Containment)
    ));
    assert!(unresolved.refined_horizon().is_none());
    let attempt = previous.refine(&inclusion, &horizon, proposed.clone(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("added observation {:?}", attempt.state()).into());
    };
    let cover = success.clock_cover().ok_or("added clock observation")?;
    assert_eq!(cover.cycles(), proposed);
    assert_eq!(cover.relation().observations(), 3);
    let point = cover.feasibility_witness();
    refined.check_witness(
        point.original().values().clone(),
        point.offset().clone(),
        &mut ledger()?,
    )?;
    let mut values = point.original().values().clone();
    values.observations.truncate(2);
    old.check_witness(values, point.offset().clone(), &mut ledger()?)?;
    assert_eq!(success.horizon().mathematical().reference(), &number("4")?);
    Ok(())
}

/// Narrowing only independent sample errors exercises the original broad-image solving path.
/// ´test:integration:lc-relation-both-timing-refinement-independent-errors´
#[test]
fn lc_relation_both_timing_refinement_independent_errors() -> TestResult {
    let pi = pi()?;
    for known in [true, false] {
        let old = timing(known, false, false)?;
        let refined = timing(known, false, true)?;
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
                return Err(format!("independent error refinement {:?}", attempt.state()).into());
            };
            included_timing(
                &previous,
                success.shared_timing_cover().ok_or("broad time image")?,
            )?;
            assert!(
                !success
                    .finite_horizon_margin()
                    .ok_or("finite broad-path margin")?
                    .is_negative()
            );
        }
    }
    Ok(())
}

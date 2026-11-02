// SPDX-License-Identifier: AGPL-3.0-only
//! Tightened and added-pair families have independent inclusion and original-clock oracles.

mod failures;
mod support;

use super::super::support::{TestResult, integer, ledger, number};
use super::super::time::{oracle::Pi, tail};
use super::support as pairs;
use nrz_bindings::{LcHorizonValue, LcRefinementState, LcZeroTimes};
use nrz_matrix::{LcHorizonProjection, LcTimeProjection};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// The two known-phase radii retain the original clock, exact branch and complete zero families.
/// ´test:integration:lc-relation-tightened-horizon´
#[test]
fn lc_relation_tightened_horizon() -> TestResult {
    let previous = pairs::cover(true, "1/20")?;
    let old_horizon = previous.forward_horizon(&mut ledger()?)?;
    let refined = pairs::relation(pairs::inputs(true, "1/40")?)?;
    let inclusion = previous
        .relation()
        .refinement_inputs(&refined, &mut ledger()?)?;
    let attempt = previous.refine(
        &inclusion,
        &old_horizon,
        previous.cycles().to_vec(),
        &mut ledger()?,
    )?;
    let LcRefinementState::Refined(success) = attempt.state() else {
        return Err("successful tightening".into());
    };
    assert_eq!(
        success.cover().rate(),
        &[number("31875/16")?, number("32125/16")?]
    );
    assert_eq!(
        success.cover().relation().epoch(),
        previous.relation().epoch()
    );
    assert_eq!(success.cover().cycles(), previous.cycles());
    for (value, coefficient) in [
        (old_horizon.value(), "8/125"),
        (success.horizon().value(), "16/125"),
    ] {
        let LcHorizonValue::Finite(endpoint) = value else {
            return Err("finite tightened horizon".into());
        };
        assert_eq!(
            endpoint.coefficients(),
            (&number("0")?, &number(coefficient)?)
        );
    }
    let old_times = previous.zero_times(&mut ledger()?)?;
    let new_times = success.cover().zero_times(&mut ledger()?)?;
    assert!(old_times.relation().shares_allocation(previous.relation()));
    assert!(
        old_times
            .pairs_cover()
            .ok_or("retained pair source")?
            .shares_allocation(&previous)
    );
    assert_eq!(
        new_times
            .pairs_cover()
            .ok_or("refined pair source")?
            .cycles(),
        previous.cycles()
    );
    assert!(new_times.relation().shares_allocation(&refined));
    let oracle = Pi::new()?;
    render::<f32>(
        &old_times,
        &new_times,
        &old_horizon,
        success.horizon(),
        &oracle,
    )?;
    render::<f64>(
        &old_times,
        &new_times,
        &old_horizon,
        success.horizon(),
        &oracle,
    )
}

/// One prepared scalar pi serves every endpoint and both distinct horizon values in a format.
fn render<P: Scalar + 'static>(
    old: &LcZeroTimes,
    new: &LcZeroTimes,
    old_horizon: &nrz_bindings::LcHorizon,
    new_horizon: &nrz_bindings::LcHorizon,
    oracle: &Pi,
) -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = super::super::super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
    for (times, horizon, index, box_oracle, start, horizon_coefficient) in [
        (
            old,
            old_horizon,
            64,
            [(336, 5375), (1008, 15875)],
            (1024, 16125),
            (8, 125),
        ),
        (
            new,
            new_horizon,
            128,
            [(4064, 32125), (4064, 31875)],
            (4096, 32125),
            (16, 125),
        ),
    ] {
        let exact_tail = tail(times, index)?.certify_feasibility(&mut ledger()?)?;
        assert_eq!(
            exact_tail.mathematical().last_separate_index(),
            &integer(index - 1)?
        );
        let native = LcTimeProjection::new(times.clone(), pi.clone(), &mut ledger()?)?;
        let last = native.member(
            integer(index - 1)?,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        for (endpoint, (top, bottom)) in last.endpoints().iter().zip(box_oracle) {
            oracle.contains(*endpoint, top, bottom)?;
        }
        let tail = native.tail(
            exact_tail,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        oracle.contains(tail.start(), start.0, start.1)?;
        let horizon = LcHorizonProjection::new(
            horizon.clone(),
            pi.clone(),
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        oracle.contains(
            horizon.enclosure().ok_or("finite horizon")?,
            horizon_coefficient.0,
            horizon_coefficient.1,
        )?;
    }
    Ok(())
}

/// Generated tightenings and added pairs certify both exact set inclusions and a no-earlier horizon.
/// ´test:integration:lc-relation-refinement-generated-inclusions´
#[test]
fn lc_relation_refinement_generated_inclusions() -> TestResult {
    for known in [false, true] {
        let previous = pairs::cover(known, "1/20")?;
        let old_horizon = previous.forward_horizon(&mut ledger()?)?;
        for denominator in [40, 60, 100, 200] {
            let radius = format!("1/{denominator}");
            for added in [false, true] {
                let refined = support::refined(known, &radius, added)?;
                let inclusion = previous
                    .relation()
                    .refinement_inputs(&refined, &mut ledger()?)?;
                let mut cycles = previous.cycles().to_vec();
                if added {
                    cycles.push(integer(1)?);
                }
                let attempt = previous.refine(&inclusion, &old_horizon, cycles, &mut ledger()?)?;
                let LcRefinementState::Refined(success) = attempt.state() else {
                    return Err("generated refinement".into());
                };
                assert!(attempt.previous().shares_allocation(&previous));
                assert!(attempt.inclusion().refined().shares_allocation(&refined));
                assert_eq!(success.cover().phase_gauge(), previous.phase_gauge());
                for quantity in [
                    nrz_bindings::LcQuantity::Inductance,
                    nrz_bindings::LcQuantity::Capacitance,
                    nrz_bindings::LcQuantity::VoltageAmplitude,
                    nrz_bindings::LcQuantity::Energy,
                ] {
                    assert_eq!(
                        success.cover().free_directions(quantity)?,
                        previous.free_directions(quantity)?
                    );
                }
                assert!(
                    success
                        .finite_horizon_margin()
                        .is_some_and(|margin| !margin.is_negative())
                );
                support::contained(previous.rate(), success.cover().rate())?;
                let products = success.product_inclusion();
                support::contained(&products[0], &products[1])?;
                if known {
                    let radius = number(&radius)?;
                    let time = number(if added { "1/250" } else { "2/625" })?;
                    let delta = radius.div(&time, &mut ledger()?)?;
                    assert_eq!(
                        success.cover().rate(),
                        &[
                            number("2000")?.sub(&delta, &mut ledger()?)?,
                            number("2000")?.add(&delta, &mut ledger()?)?,
                        ]
                    );
                }
            }
        }
    }
    Ok(())
}

/// Exact measurements are a compatible tightening to a singleton rate and an infinite horizon.
/// ´test:integration:lc-relation-refinement-to-infinite-horizon´
#[test]
fn lc_relation_refinement_to_infinite_horizon() -> TestResult {
    let previous = pairs::cover(true, "1/20")?;
    let old_horizon = previous.forward_horizon(&mut ledger()?)?;
    let refined = pairs::relation(pairs::inputs(true, "0")?)?;
    let inclusion = previous
        .relation()
        .refinement_inputs(&refined, &mut ledger()?)?;
    let attempt = previous.refine(
        &inclusion,
        &old_horizon,
        previous.cycles().to_vec(),
        &mut ledger()?,
    )?;
    let LcRefinementState::Refined(success) = attempt.state() else {
        return Err("exact-frequency refinement".into());
    };
    assert_eq!(success.cover().rate(), &[number("2000")?, number("2000")?]);
    assert!(matches!(
        success.horizon().value(),
        LcHorizonValue::Infinite
    ));
    let times = success.cover().zero_times(&mut ledger()?)?;
    assert!(matches!(
        times.overlap_tail(integer(1)?, &mut ledger()?),
        nrz_bindings::LcTailOutcome::SeparatedInfinite(_)
    ));
    Ok(())
}

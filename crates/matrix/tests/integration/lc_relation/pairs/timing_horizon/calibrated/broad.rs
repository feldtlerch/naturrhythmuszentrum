// SPDX-License-Identifier: AGPL-3.0-only
//! Both formats enclose original broad-calibration futures against independent Machin arithmetic.

use super::support::interval;
use super::{Pi, TestResult, cycles, input, ledger, pi_certificate, render};
use nrz_bindings::{
    LcClockRefinementState, LcQuantity as Q, LcRelation, LcSharedClock, LcSharedClockPhase,
    LcSharedMeasurements,
};

/// The physical phases contain every measured value plus its shared offset.
fn phase(tight: bool, channel: Option<Q>) -> TestResult<LcSharedClockPhase> {
    let old = super::support::phase(tight, channel)?;
    let mut inputs = old.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Phase(_)));
    for index in 0..2 {
        inputs.push(input(Q::Phase(index), ["-10", "10"])?);
    }
    let relation = LcRelation::new(old.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    let offset = interval(if tight {
        ["-1/8", "1/8"]
    } else {
        ["-1/4", "1/4"]
    })?;
    let clock = LcSharedClock::new(
        relation,
        old.clock().nominal_times().to_vec(),
        &offset,
        &mut ledger()?,
    )?;
    Ok(LcSharedClockPhase::new(
        clock,
        old.samples().to_vec(),
        &offset,
        &mut ledger()?,
    )?)
}

/// Original sources and conditional witnesses survive binary32 and binary64 future publication.
/// ´test:integration:lc-relation-broad-calibrated-native-future´
#[test]
fn lc_relation_broad_calibrated_native_future() -> TestResult {
    let mut fixtures = Vec::new();
    for tight in [false, true] {
        let source = phase(tight, None)?;
        let cover = source.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
        let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
        assert!(horizon.relation().shares_allocation(source.relation()));
        assert!(
            horizon
                .calibrated_future()
                .ok_or("physical future")?
                .clock_phase_cover()
                .ok_or("original broad cover")?
                .shares_allocation(&cover)
        );
        fixtures.push((horizon, if tight { "2" } else { "1" }, "5/4"));
    }
    let old = super::support::amplitude(false, Q::VoltageAmplitude)?;
    let source = LcSharedMeasurements::new(
        phase(false, Some(old.channel()))?,
        old.channel(),
        old.samples().to_vec(),
        &interval(["1/2", "3/2"])?,
        &mut ledger()?,
    )?;
    let cover = source.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
    assert!(
        horizon
            .calibrated_future()
            .ok_or("physical amplitude future")?
            .shared_measurement_cover()
            .ok_or("original amplitude cover")?
            .shares_allocation(&cover)
    );
    fixtures.push((horizon, "1", "5/4"));
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// A refinement entering the broad path retains the same future reference and an explicit infinite answer.
/// ´test:integration:lc-relation-native-refinement-enters-broad´
#[test]
fn lc_relation_native_refinement_enters_broad() -> TestResult {
    let old = super::support::phase(false, None)?;
    let new = LcSharedClockPhase::new(
        old.clock().clone(),
        vec![interval(["2", "2"])?, interval(["4", "4"])?],
        &interval(["-1/8", "1/8"])?,
        &mut ledger()?,
    )?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let cover = old.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
    let attempt = cover.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("native entering broad {:?}", attempt.state()).into());
    };
    assert!(matches!(
        success.horizon().value(),
        nrz_bindings::LcHorizonValue::Infinite
    ));
    assert!(
        success
            .horizon()
            .relation()
            .shares_allocation(new.relation())
    );
    assert_eq!(
        success.horizon().mathematical().reference(),
        horizon.mathematical().reference()
    );
    let fixtures = [(horizon, "2", "3/2"), (success.horizon().clone(), "0", "0")];
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

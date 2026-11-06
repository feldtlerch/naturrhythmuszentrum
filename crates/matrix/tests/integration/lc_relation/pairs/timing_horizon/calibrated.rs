// SPDX-License-Identifier: AGPL-3.0-only
//! Both native formats enclose the independent affine-pi calibrated future oracle.

mod broad;
mod support;
use super::support::{cycles, render};
use super::{Pi, TestResult, input, ledger, number, pi_certificate};
use nrz_bindings::{LcClockRefinementState, LcQuantity as Q};

/// Phase and amplitude calibration retain original future evidence through both scalar projections.
/// ´test:integration:lc-relation-calibrated-future-native-horizon´
#[test]
fn lc_relation_calibrated_future_native_horizon() -> TestResult {
    let mut fixtures = Vec::new();
    for tight in [false, true] {
        let phase = support::phase(tight, None)?;
        let cover = phase.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
        let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
        assert!(horizon.relation().shares_allocation(phase.relation()));
        assert!(
            horizon
                .calibrated_future()
                .ok_or("original future")?
                .clock_phase_cover()
                .ok_or("phase source")?
                .shares_allocation(&cover)
        );
        fixtures.push((horizon, if tight { "4" } else { "2" }, "3/2"));
    }
    let old = support::amplitude(false, Q::VoltageAmplitude)?;
    let new = support::amplitude(true, Q::VoltageAmplitude)?;
    let cover = old.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let attempt = cover.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("native calibrated refinement {:?}", attempt.state()).into());
    };
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
    fixtures.push((horizon, "2", "3/2"));
    fixtures.push((success.horizon().clone(), "4", "3/2"));
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

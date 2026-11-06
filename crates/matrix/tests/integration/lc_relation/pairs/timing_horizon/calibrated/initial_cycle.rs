// SPDX-License-Identifier: AGPL-3.0-only
//! An original-implied initial strip retains complete signed-time cycles in both native futures.

use super::support::interval;
use super::{Pi, TestResult, cycles, input, ledger, number, pi_certificate, render};
use nrz_bindings::{LcQuantity as Q, LcRelation, LcSharedClock, LcSharedClockPhase};

/// Both measured and physical declarations survive the independently checked auxiliary cycle proof.
fn source() -> TestResult<LcSharedClockPhase> {
    let mut inputs = vec![
        input(Q::Rate, ["1", "3"])?,
        input(Q::InitialPhase, ["0", "0"])?,
    ];
    for (index, (time, phase)) in [
        (["-13/4", "-11/4"], ["-13/2", "-11/2"]),
        (["11/4", "13/4"], ["11/2", "13/2"]),
    ]
    .into_iter()
    .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    let relation = LcRelation::new(
        "native signed-cycle question".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let offset = interval(["-1/8", "1/8"])?;
    let clock = LcSharedClock::new(
        relation,
        vec![number("-3")?, number("3")?],
        &offset,
        &mut ledger()?,
    )?;
    Ok(LcSharedClockPhase::new(
        clock,
        vec![interval(["-10", "10"])?, interval(["-10", "10"])?],
        &offset,
        &mut ledger()?,
    )?)
}

/// The new complete cycle case retains its original source and the independent `6*pi` future oracle.
/// ´test:integration:lc-relation-original-implied-initial-native´
#[test]
fn lc_relation_original_implied_initial_native() -> TestResult {
    let source = source()?;
    let cover = source.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
    assert!(horizon.relation().shares_allocation(source.relation()));
    let future = horizon
        .calibrated_future()
        .ok_or("original signed-cycle future")?;
    assert!(
        future
            .clock_phase_cover()
            .ok_or("complete original cover")?
            .shares_allocation(&cover)
    );
    assert_eq!(
        horizon.extremal_parameters(),
        &[
            [number("0")?, number("11/6")?],
            [number("0")?, number("13/6")?]
        ]
    );
    for point in future
        .phase_witnesses()
        .ok_or("full-original extremizers")?
    {
        source.check_witness(
            point.original().original().values().clone(),
            point.original().offset().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    let fixtures = [(horizon, "6", "0")];
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

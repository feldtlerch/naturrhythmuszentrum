// SPDX-License-Identifier: AGPL-3.0-only
//! Narrow original total timestamps preserve both timing layers in actual-time horizon publication.

use super::support::{both_timing, cycles, finite, render, work_boundary};
use super::{Pi, TestResult, input, ledger, number, pi_certificate};
use nrz_bindings::{
    LcClockPhaseParameters, LcInterval, LcQuantity as Q, LcRelation, LcSharedTiming,
};

/// Both native formats retain narrowed original times and the common-offset physical extrema.
/// ´test:integration:lc-relation-narrower-both-timing-horizon´
#[test]
fn lc_relation_narrower_both_timing_horizon() -> TestResult {
    let pi = pi_certificate()?;
    let mut fixtures = Vec::new();
    for (known, width, coefficient, offset) in [
        (true, ["0", "800/39999"], "39999/400", "0"),
        (false, ["-100/833", "200/2499"], "2499/100", "3/2"),
    ] {
        let source = question(known)?;
        let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        let horizon = cover.forward_horizon(&mut ledger()?)?;
        assert!(horizon.relation().shares_allocation(source.relation()));
        assert!(
            horizon
                .shared_timing_cover()
                .ok_or("original two-layer horizon source")?
                .timing()
                .shares_allocation(&source)
        );
        assert_eq!(horizon.mathematical().reference(), &number("201/100")?);
        assert_eq!(
            horizon.mathematical().width(),
            &[number(width[0])?, number(width[1])?]
        );
        finite(&horizon, offset, coefficient)?;
        for point in horizon.extremal_parameters() {
            let offset = cover.offset_at_phase(&point[1], &point[0], &mut ledger()?)?[0].clone();
            let witness = cover.witness_with_phase(
                &LcClockPhaseParameters {
                    rate: point[1].clone(),
                    initial_phase: point[0].clone(),
                    offset,
                    gauge: number("0")?,
                    ratio: number("1")?,
                    voltage: number("1")?,
                },
                &mut ledger()?,
            )?;
            source.check_witness(
                witness.original().values().clone(),
                witness.offset().clone(),
                &mut ledger()?,
            )?;
        }
        work_boundary(|budget| cover.forward_horizon(budget))?;
        fixtures.push((horizon, coefficient, offset));
    }
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// Only original total timestamps narrow; both measurement-error declarations retain their original supports.
fn question(known: bool) -> TestResult<LcSharedTiming> {
    let base = both_timing()?;
    let mut inputs = base.relation().inputs().to_vec();
    inputs.retain(|value| {
        let keep_time = !matches!(value.quantity(), Q::Time(_));
        let keep_phase = known || value.quantity() != Q::InitialPhase;
        keep_time && keep_phase
    });
    for (index, time) in [["99/100", "101/100"], ["199/100", "201/100"]]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
    }
    let relation = LcRelation::new(
        base.relation().epoch().into(),
        base.relation().observations(),
        inputs,
        &mut ledger()?,
    )?;
    let errors = base
        .errors()
        .iter()
        .map(interval)
        .collect::<TestResult<Vec<_>>>()?;
    Ok(LcSharedTiming::new(
        relation,
        base.nominal_times().to_vec(),
        &errors,
        &interval(base.offset())?,
        &mut ledger()?,
    )?)
}

/// Original exact error supports are retained without a marginal-box replacement.
fn interval(range: &[nrz_bindings::readings::algebra::Rational; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(range[0].clone().into()),
        Some(range[1].clone().into()),
        &mut ledger()?,
    )?)
}

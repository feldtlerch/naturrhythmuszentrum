// SPDX-License-Identifier: AGPL-3.0-only
//! Future reported readings retain the same original shared offset and explicit coordinate semantics.

use super::support::{clock, parameters, work_boundary};
use super::{TestResult, cycles, input, ledger, number, pi};
use nrz_bindings::{
    ExactConstant, LcHorizonCoordinate, LcHorizonValue, LcInterval, LcQuantity as Q, LcRelation,
    LcSharedTiming,
};

/// A reported reading uses the original shared offset and has a distinct query intercept.
/// ´test:integration:lc-relation-reported-clock-coordinate´
#[test]
fn lc_relation_reported_clock_coordinate() -> TestResult {
    let source = clock(None, ["-1/10", "1/10"], Vec::new())?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let actual = cover.forward_horizon(&mut ledger()?)?;
    let reported = cover.forward_reported_horizon(&mut ledger()?)?;
    assert_eq!(actual.coordinate(), LcHorizonCoordinate::ActualTime);
    assert_eq!(reported.coordinate(), LcHorizonCoordinate::ReportedClock);
    assert_eq!(reported.mathematical().reference(), &number("11/5")?);
    assert_eq!(
        reported.mathematical().width(),
        &[number("0")?, number("0")?]
    );
    assert!(matches!(reported.value(), LcHorizonValue::Infinite));
    assert!(
        reported
            .clock_cover()
            .ok_or("retained shared clock")?
            .clock()
            .shares_allocation(&source)
    );
    assert_ne!(actual.extremal_parameters(), reported.extremal_parameters());
    let point = cover.witness_with_phase(&parameters("2", "1/5", "-1/10", "0")?, &mut ledger()?)?;
    let actual_time =
        ExactConstant::from(number("3")?).add(&point.offset().clone().into(), &mut ledger()?)?;
    let values = point.original().values();
    let future = values.initial_phase.as_ref().ok_or("physical alpha")?.add(
        &values.rate.mul(&actual_time, &mut ledger()?)?,
        &mut ledger()?,
    )?;
    assert!(future.structurally_equal(&number("6")?.into(), &mut ledger()?)?);
    let explicit = cover.forward_reported_horizon_from(&number("3")?, &mut ledger()?)?;
    assert_eq!(explicit.mathematical().reference(), &number("3")?);
    assert!(
        cover
            .forward_reported_horizon_from(&number("2")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Both original timing layers keep their conditional reported intercept after physical timestamp narrowing.
/// ´test:integration:lc-relation-reported-both-timing-coordinate´
#[test]
fn lc_relation_reported_both_timing_coordinate() -> TestResult {
    for (known, narrow, reference, offset, coefficient) in [
        (true, false, "111/50", "4/3", "9991/600"),
        (false, false, "111/50", "4/3", "9991/600"),
        (true, true, "203/100", "-1", "39999/400"),
        (false, true, "203/100", "1", "2499/100"),
    ] {
        let source = both(known, narrow)?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        let actual = cover.forward_horizon(&mut ledger()?)?;
        let reported = cover.forward_reported_horizon(&mut ledger()?)?;
        assert_eq!(reported.coordinate(), LcHorizonCoordinate::ReportedClock);
        assert_eq!(reported.mathematical().reference(), &number(reference)?);
        assert_ne!(
            reported.mathematical().width(),
            actual.mathematical().width()
        );
        assert!(
            reported
                .shared_timing_cover()
                .ok_or("original two-layer source")?
                .timing()
                .shares_allocation(&source)
        );
        let LcHorizonValue::Finite(value) = reported.value() else {
            return Err("finite reported horizon".into());
        };
        assert_eq!(
            value.coefficients(),
            (&number(offset)?, &number(coefficient)?)
        );
    }
    Ok(())
}

/// Reported-coordinate queries spend cumulative Work without replacing either original clock owner.
/// ´test:integration:lc-relation-reported-clock-work´
#[test]
fn lc_relation_reported_clock_work() -> TestResult {
    let pi = pi()?;
    let clock =
        clock(None, ["-1/10", "1/10"], Vec::new())?.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let reference = number("3")?;
    work_boundary(|budget| clock.forward_reported_horizon(budget))?;
    work_boundary(|budget| clock.forward_reported_horizon_from(&reference, budget))?;
    for narrow in [false, true] {
        let cover = both(false, narrow)?.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        work_boundary(|budget| cover.forward_reported_horizon(budget))?;
        work_boundary(|budget| cover.forward_reported_horizon_from(&reference, budget))?;
        assert!(
            cover
                .forward_reported_horizon_from(&number("2")?, &mut ledger()?)
                .is_err()
        );
    }
    Ok(())
}

/// Original phase and error declarations remain identical when only physical timestamp restrictions narrow.
fn both(known: bool, narrow: bool) -> TestResult<LcSharedTiming> {
    let base = clock(known.then_some(["0", "0"]), ["-1/10", "1/10"], Vec::new())?;
    let mut inputs = base.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Time(_)));
    let times = if narrow {
        [["99/100", "101/100"], ["199/100", "201/100"]]
    } else {
        [["89/100", "111/100"], ["47/25", "53/25"]]
    };
    for (index, time) in times.into_iter().enumerate() {
        inputs.push(input(Q::Time(index), time)?);
    }
    let relation = LcRelation::new(
        base.relation().epoch().into(),
        base.relation().observations(),
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSharedTiming::new(
        relation,
        base.nominal_times().to_vec(),
        &[interval(["-1/100", "1/100"])?, interval(["-1/50", "1/50"])?],
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?)
}

/// Exact coherent intervals preserve the original signed error supports.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

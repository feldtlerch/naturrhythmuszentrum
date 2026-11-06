// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded forward sets retain original coupled calibration and exact query-time fibres.

use super::{TestResult, boundary, cycles, input, interval, ledger, number, pi};
use nrz_bindings::{
    LcCalibratedElements, LcCalibratedForwardCover, LcQuantity as Q, LcRelation, LcSampleTimes,
};

/// The original fixed initial phase is independent of the separate query-time window.
fn source(initial: &str) -> TestResult<nrz_bindings::LcCalibratedElementsCover> {
    let original = super::question(0)?;
    let mut inputs = original.relation().inputs().to_vec();
    inputs.push(input(Q::InitialPhase, [initial, initial])?);
    let relation = LcRelation::new(
        original.relation().epoch().into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let timing = LcSampleTimes::new(
        relation,
        original.timing().nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    let question = LcCalibratedElements::new(
        timing,
        original.samples().to_vec(),
        &interval(["1", "2"])?,
        &mut ledger()?,
    )?;
    Ok(question.cover(cycles(), &pi()?, &mut ledger()?)?)
}

/// A full original calibrated rate interval maps to one complete closed wrapped phase arc.
/// ´test:integration:lc-relation-coupled-forward-set´
#[test]
fn lc_relation_coupled_forward_set() -> TestResult {
    let source = source("0")?;
    assert_eq!(source.rate(), &[number("2")?, number("17/8")?]);
    let query = source.forward_phase_window([number("1/4")?, number("1/2")?], &mut ledger()?)?;
    assert!(query.relation().shares_allocation(source.relation()));
    assert!(
        query
            .source()
            .question()
            .shares_allocation(source.question())
    );
    assert_eq!(query.phases(), &[number("1/2")?, number("17/16")?]);
    assert_eq!(query.time_window(), &[number("1/4")?, number("1/2")?]);
    assert_eq!(query.certainty(), nrz_bindings::LcCertainty::Certified);
    assert_eq!(
        query.determinacy(),
        nrz_bindings::LcDeterminacy::DeterminedFinite
    );
    // The literal upper endpoint is below six, already below the retained lower full-turn bound.
    assert_eq!(
        number("17/16")?.compare(&number("6")?, &mut ledger()?)?,
        std::cmp::Ordering::Less
    );
    assert_eq!(
        number("6")?.compare(
            &query
                .pi_bounds()
                .contract()
                .lower
                .mul(&number("2")?, &mut ledger()?)?,
            &mut ledger()?
        )?,
        std::cmp::Ordering::Less
    );
    for (point, phase) in query.endpoint_witnesses().iter().zip(["1/2", "17/16"]) {
        assert_eq!(point.phase(), &number(phase)?);
        original_query(&query, point)?;
    }
    Ok(())
}

/// Endpoint and interior phases have original physical solutions and compatible exact query times.
/// ´test:integration:lc-relation-coupled-forward-witnesses´
#[test]
fn lc_relation_coupled_forward_witnesses() -> TestResult {
    let source = source("0")?;
    let query = source.forward_phase_window([number("1/4")?, number("1/2")?], &mut ledger()?)?;
    for (phase, rate, time) in [
        ("1/2", "2", "1/4"),
        ("3/4", "2", "3/8"),
        ("1", "2", "1/2"),
        ("17/16", "17/8", "1/2"),
    ] {
        let point = query.witness_at_phase(&number(phase)?, &mut ledger()?)?;
        assert_eq!(
            point.original().original().values().rate.as_rational(),
            Some(&number(rate)?)
        );
        assert_eq!(point.time(), &number(time)?);
        assert_eq!(point.phase(), &number(phase)?);
        assert_eq!(
            number(rate)?.mul(&number(time)?, &mut ledger()?)?,
            number(phase)?
        );
        original_query(&query, &point)?;
    }
    Ok(())
}

/// All original equations and measurements, and the separate query equation, are checked independently.
fn original_query(
    query: &LcCalibratedForwardCover,
    point: &nrz_bindings::LcCalibratedForwardWitness,
) -> TestResult {
    let original = point.original();
    assert!(
        original
            .question()
            .shares_allocation(query.source().question())
    );
    assert!(
        original
            .original()
            .relation()
            .shares_allocation(query.relation())
    );
    query.source().question().check_witness(
        original.original().values().clone(),
        original.calibration().clone(),
        &mut ledger()?,
    )?;
    let values = original.original().values();
    assert_eq!(
        values
            .initial_phase
            .as_ref()
            .and_then(nrz_bindings::ExactConstant::as_rational),
        Some(&number("0")?)
    );
    let rate = values.rate.as_rational().ok_or("rational rate")?;
    assert_eq!(&rate.mul(point.time(), &mut ledger()?)?, point.phase());
    assert_ne!(
        point
            .time()
            .compare(&query.time_window()[0], &mut ledger()?)?,
        std::cmp::Ordering::Less
    );
    assert_ne!(
        point
            .time()
            .compare(&query.time_window()[1], &mut ledger()?)?,
        std::cmp::Ordering::Greater
    );
    Ok(())
}

/// Unknown initial phase, nonzero fixed phase, unsupported wraps and crossed query supports refuse.
/// ´test:integration:lc-relation-coupled-forward-refusals´
#[test]
fn lc_relation_coupled_forward_refusals() -> TestResult {
    let source = source("0")?;
    for times in [["0", "1/2"], ["1/2", "1/4"], ["-1/4", "1/2"], ["1", "4"]] {
        assert!(
            source
                .forward_phase_window([number(times[0])?, number(times[1])?], &mut ledger()?)
                .is_err()
        );
    }
    let query = source.forward_phase_window([number("1/4")?, number("1/2")?], &mut ledger()?)?;
    for phase in ["0", "1/4", "2"] {
        assert!(
            query
                .witness_at_phase(&number(phase)?, &mut ledger()?)
                .is_err()
        );
    }
    let unknown = super::question(0)?.cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(
        unknown
            .forward_phase_window([number("1/4")?, number("1/2")?], &mut ledger()?)
            .is_err()
    );
    let nonzero = self::source("1/8")?;
    assert!(
        nonzero
            .forward_phase_window([number("1/4")?, number("1/2")?], &mut ledger()?)
            .is_err()
    );
    original_query(
        &query,
        &query.witness_at_phase(&number("3/4")?, &mut ledger()?)?,
    )
}

/// Full query publication and original reconstruction preserve exact caller Work and one-short refusal.
/// ´test:integration:lc-relation-coupled-forward-work´
#[test]
fn lc_relation_coupled_forward_work() -> TestResult {
    let source = source("0")?;
    let times = [number("1/4")?, number("1/2")?];
    boundary(|budget| source.forward_phase_window(times.clone(), budget))?;
    let query = source.forward_phase_window(times, &mut ledger()?)?;
    let phase = number("17/16")?;
    boundary(|budget| query.witness_at_phase(&phase, budget))
}

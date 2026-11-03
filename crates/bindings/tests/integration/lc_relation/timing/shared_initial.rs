// SPDX-License-Identifier: AGPL-3.0-only
//! Both timing layers retain physical alpha beside the conditional common-offset fibre.

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    Budget, ExactConstant, LcClockPhaseParameters, LcInput, LcInterval, LcPhaseGauge,
    LcQuantity as Q, LcRelation, LcSharedTiming, Limits, LimitsOverride,
};

/// Original physical total boxes enclose the complete independent-plus-shared time image.
fn question(initial: Option<[&str; 2]>, extra: Vec<LcInput>) -> TestResult<LcSharedTiming> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if let Some(initial) = initial {
        inputs.push(input(Q::InitialPhase, initial)?);
    }
    for (index, (range, phase)) in [(["89/100", "111/100"], "2"), (["47/25", "53/25"], "4")]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), range)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    inputs.extend(extra);
    let relation = LcRelation::new(
        "both time layers and physical phase".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSharedTiming::new(
        relation,
        vec![number("1")?, number("2")?],
        &[interval(["-1/100", "1/100"])?, interval(["-1/50", "1/50"])?],
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?)
}

/// Each uncertainty coordinate has its own independently declared exact support.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// A physical initial phase and a single shared offset select one joint original tuple.
fn parameters(
    rate: &str,
    phase: &str,
    offset: &str,
    gauge: &str,
) -> TestResult<LcClockPhaseParameters> {
    Ok(LcClockPhaseParameters {
        rate: number(rate)?,
        initial_phase: number(phase)?,
        offset: number(offset)?,
        gauge: number(gauge)?,
        ratio: number("1")?,
        voltage: number("1")?,
    })
}

/// Original alpha and one common offset retain their exact conditional correlation with both time layers.
/// ´test:integration:lc-relation-both-timing-unknown-initial-phase´
#[test]
fn lc_relation_both_timing_unknown_initial_phase() -> TestResult {
    let timing = question(None, Vec::new())?;
    let cover = timing.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(timing.initial_phase_support(), None);
    assert_eq!(cover.rate(), &[number("200/103")?, number("200/97")?]);
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_eq!(cover.cycles(), cycles());
    assert!(cover.timing().shares_allocation(&timing));
    assert!(cover.relation().shares_allocation(timing.relation()));
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-11/50")?, number("11/50")?]
    );
    assert_eq!(
        cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/10")?, number("1/10")?]
    );
    for (phase, offset) in [("11/50", "-1/10"), ("-11/50", "1/10")] {
        assert_eq!(
            cover.offset_at_phase(&number("2")?, &number(phase)?, &mut ledger()?)?,
            [number(offset)?, number(offset)?]
        );
        let point =
            cover.witness_with_phase(&parameters("2", phase, offset, "0")?, &mut ledger()?)?;
        assert!(point.timing().shares_allocation(&timing));
        assert_eq!(
            point.original().coordinate(Q::InitialPhase),
            Some(&number(phase)?.into())
        );
        timing.check_witness(
            point.original().values().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    assert_eq!(
        cover.offset_at_phase(&number("2")?, &number("0")?, &mut ledger()?)?,
        [number("-1/100")?, number("1/100")?]
    );
    assert!(
        cover
            .witness_with_phase(&parameters("2", "11/50", "1/10", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .offset_at_phase(&number("2")?, &number("1/4")?, &mut ledger()?)
            .is_err()
    );
    assert!(cover.bounds(Q::InitialPhase, &mut ledger()?).is_err());
    assert!(cover.determinacy(Q::InitialPhase).is_err());
    assert!(cover.free_directions(Q::InitialPhase).is_err());
    Ok(())
}

/// Bounded and fixed physical phases keep every original restriction and the correct gauge.
/// ´test:integration:lc-relation-both-timing-bounded-initial-phase´
#[test]
fn lc_relation_both_timing_bounded_initial_phase() -> TestResult {
    let timing = question(
        Some(["0", "1/5"]),
        vec![input(Q::InitialPhase, ["1/20", "3/20"])?],
    )?;
    let cover = timing.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        timing.initial_phase_support(),
        Some(&[number("1/20")?, number("3/20")?])
    );
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::Fixed);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("1/20")?, number("3/20")?]
    );
    assert_eq!(
        cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-17/200")?, number("-3/200")?]
    );
    for (phase, offset) in [("1/20", "-3/200"), ("1/10", "-1/20"), ("3/20", "-17/200")] {
        let point =
            cover.witness_with_phase(&parameters("2", phase, offset, "0")?, &mut ledger()?)?;
        timing.check_witness(
            point.original().values().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    assert!(
        cover
            .witness_with_phase(&parameters("2", "0", "0", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness_with_phase(&parameters("2", "1/10", "-1/20", "1")?, &mut ledger()?)
            .is_err()
    );
    let fixed = question(Some(["1/10", "1/10"]), Vec::new())?.pairs_cover(
        cycles(),
        &pi()?,
        &mut ledger()?,
    )?;
    let bound = fixed.bounds(Q::InitialPhase, &mut ledger()?)?;
    assert_eq!(bound.lower(), Some(&number("1/10")?.into()));
    assert_eq!(bound.lower(), bound.upper());
    assert_eq!(
        fixed.determinacy(Q::InitialPhase)?,
        nrz_bindings::LcDeterminacy::DeterminedFinite
    );
    assert_eq!(
        fixed.free_directions(Q::InitialPhase)?,
        [] as [nrz_bindings::LcFreeDirection; 0]
    );
    Ok(())
}

/// Every original integer gauge restores physical alpha, observations and both timing equations.
/// ´test:integration:lc-relation-both-timing-initial-gauges´
#[test]
fn lc_relation_both_timing_initial_gauges() -> TestResult {
    let timing = question(None, Vec::new())?;
    let cover = timing.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let turn = ExactConstant::named(nrz_cas::syntax::Constant::Pi, &mut ledger()?)?
        .mul(&number("2")?.into(), &mut ledger()?)?;
    for gauge in ["-2", "0", "3"] {
        let point =
            cover.witness_with_phase(&parameters("2", "11/50", "-1/10", gauge)?, &mut ledger()?)?;
        let expected = ExactConstant::from(number("11/50")?).add(
            &turn.mul(&number(gauge)?.into(), &mut ledger()?)?,
            &mut ledger()?,
        )?;
        assert!(
            point
                .original()
                .coordinate(Q::InitialPhase)
                .ok_or("physical alpha")?
                .structurally_equal(&expected, &mut ledger()?)?
        );
        for observation in &point.original().values().observations {
            assert_eq!(observation.cycle.as_ref(), Some(&number(gauge)?));
        }
        timing.check_witness(
            point.original().values().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Wrong cycles and exact Work exhaustion preserve the original two-layer question.
/// ´test:integration:lc-relation-both-timing-initial-refusals-work´
#[test]
fn lc_relation_both_timing_initial_refusals_work() -> TestResult {
    let timing = question(None, Vec::new())?;
    let pi = pi()?;
    assert!(
        timing
            .pairs_cover(vec![number("0")?, number("1")?], &pi, &mut ledger()?)
            .is_err()
    );
    boundary(|budget| timing.pairs_cover(cycles(), &pi, budget))?;
    let cover = timing.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let parameters = parameters("2", "11/50", "-1/10", "0")?;
    boundary(|budget| cover.witness_with_phase(&parameters, budget))?;
    boundary(|budget| cover.phase_at_rate(&parameters.rate, budget))?;
    boundary(|budget| cover.offset_at_phase(&parameters.rate, &parameters.initial_phase, budget))?;
    boundary(|budget| cover.offset_at_rate(&parameters.rate, budget))?;
    boundary(|budget| cover.forward_horizon(budget))?;
    Ok(())
}

/// Measured exact work includes the caller prefix on both successful and refused attempts.
fn boundary<T>(
    operation: impl Fn(&mut Budget) -> Result<T, nrz_bindings::LcRelationError>,
) -> TestResult {
    let mut measured = ledger()?;
    operation(&mut measured)?;
    let work = measured.usage().work;
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, nrz_cas::Location::new("both timing initial prefix"))?;
        assert_eq!(operation(&mut budget).is_ok(), success);
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

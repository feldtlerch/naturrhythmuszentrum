// SPDX-License-Identifier: AGPL-3.0-only
//! Shared phase offsets preserve their joint initial-phase and per-sample measurement fibres.

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi, point};
use nrz_bindings::{
    LcInterval, LcPairParameters, LcPhaseGauge, LcPhaseParameters, LcQuantity as Q, LcRelation,
    LcRelationError, LcSampleTimes, LcSharedPhase,
};

/// Separate phase intervals and a common offset differ from either layer alone.
/// ´test:integration:lc-relation-phase-both-layers´
#[test]
fn lc_relation_phase_both_layers() -> TestResult {
    let phase = question(true, true, Some(["0", "0"]))?;
    let cover = phase.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("197/100")?, number("203/100")?]);
    let independent =
        question(true, false, Some(["0", "0"]))?.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        independent.rate(),
        &[number("199/100")?, number("201/100")?]
    );
    let shared =
        question(false, true, Some(["0", "0"]))?.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(shared.rate(), &[number("2")?, number("2")?]);
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::Fixed);
    assert!(cover.phase().shares_allocation(&phase));
    for (rate, offset) in [("197/100", "-1/25"), ("203/100", "1/25")] {
        assert_eq!(
            cover.offset_at_phase(&number(rate)?, &number("0")?, &mut ledger()?)?,
            [number(offset)?, number(offset)?]
        );
        let witness = cover.witness(&parameters(rate, "0", offset)?, &mut ledger()?)?;
        assert!(witness.phase().shares_allocation(&phase));
        assert!(
            witness
                .original()
                .relation()
                .shares_allocation(phase.relation())
        );
        phase.check_witness(
            witness.original().values().clone(),
            witness.offset().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Replacing the common phase error with independent total boxes invents feasible tuples.
/// ´test:integration:lc-relation-phase-marginal-control´
#[test]
fn lc_relation_phase_marginal_control() -> TestResult {
    let phase = question(true, true, Some(["0", "0"]))?;
    let cover = phase.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let marginal = phase
        .timing()
        .pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(marginal.rate(), &[number("97/50")?, number("103/50")?]);
    let mut values = point("41/20", ["1", "2"])?;
    for (observation, phase) in values.observations.iter_mut().zip(["41/20", "41/10"]) {
        observation.phase = Some(number(phase)?.into());
    }
    phase
        .relation()
        .check_witness(values.clone(), &mut ledger()?)?;
    assert!(matches!(
        cover.phase_at_rate(&number("41/20")?, &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    for offset in ["0", "1/20", "1/10"] {
        assert!(
            phase
                .check_witness(values.clone(), number(offset)?, &mut ledger()?)
                .is_err()
        );
    }
    Ok(())
}

/// Unknown or bounded physical initial phase retains its correlation with one shared offset.
/// ´test:integration:lc-relation-phase-initial-offset-fibre´
#[test]
fn lc_relation_phase_initial_offset_fibre() -> TestResult {
    let unknown = question(true, true, None)?;
    let cover = unknown.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-11/100")?, number("11/100")?]
    );
    assert_eq!(
        cover.offset_at_phase(&number("2")?, &number("11/100")?, &mut ledger()?)?,
        [number("1/10")?, number("1/10")?]
    );
    cover.witness(&parameters("2", "11/100", "1/10")?, &mut ledger()?)?;
    for gauge in ["-1", "1"] {
        let mut shifted = parameters("2", "11/100", "1/10")?;
        shifted.physical.gauge = number(gauge)?;
        let witness = cover.witness(&shifted, &mut ledger()?)?;
        unknown.check_witness(
            witness.original().values().clone(),
            witness.offset().clone(),
            &mut ledger()?,
        )?;
    }
    assert!(
        cover
            .witness(&parameters("2", "11/100", "-1/10")?, &mut ledger()?)
            .is_err()
    );
    let bounded = question(true, true, Some(["-1/50", "1/50"]))?;
    let cover = bounded.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/50")?, number("1/50")?]
    );
    assert_eq!(
        cover.offset_at_phase(&number("2")?, &number("1/50")?, &mut ledger()?)?,
        [number("1/100")?, number("3/100")?]
    );
    cover.witness(&parameters("2", "1/50", "3/100")?, &mut ledger()?)?;
    assert!(
        cover
            .witness(&parameters("2", "11/100", "1/10")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Shared phase uncertainty composes with retained independent time intervals.
/// ´test:integration:lc-relation-phase-uncertain-times´
#[test]
fn lc_relation_phase_uncertain_times() -> TestResult {
    let original = original(Some(["0", "0"]))?;
    let mut inputs: Vec<_> = original
        .inputs()
        .iter()
        .filter(|input| !matches!(input.quantity(), Q::Time(_)))
        .cloned()
        .collect();
    for (index, range) in [["99/100", "101/100"], ["199/100", "201/100"]]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), range)?);
    }
    let relation = LcRelation::new(original.epoch().into(), 2, inputs, &mut ledger()?)?;
    let timing = LcSampleTimes::new(relation, vec![number("1")?, number("2")?], &mut ledger()?)?;
    let phase = LcSharedPhase::new(
        timing,
        samples(true)?,
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?;
    let cover = phase.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    // The lower endpoint is the phase-difference minimum over the time-difference maximum.
    // The upper endpoint is the later sample's maximum true phase over its earliest time.
    assert_eq!(cover.rate(), &[number("197/102")?, number("412/199")?]);
    for (rate, offset) in [("197/102", "-333/3400"), ("412/199", "1/10")] {
        let witness = cover.witness(&parameters(rate, "0", offset)?, &mut ledger()?)?;
        phase.check_witness(
            witness.original().values().clone(),
            witness.offset().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Unsupported original-image restrictions and exhausted Work retain explicit refusals.
/// ´test:integration:lc-relation-phase-refusals-work´
#[test]
fn lc_relation_phase_refusals_work() -> TestResult {
    use nrz_bindings::{Budget, Limits, LimitsOverride};
    const FIRST: usize = 0;
    let phase = question(true, true, Some(["0", "0"]))?;
    let mut inputs = phase.relation().inputs().to_vec();
    inputs.push(input(Q::Phase(FIRST), ["2", "2"])?);
    let narrow = LcRelation::new(phase.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    let timing = LcSampleTimes::new(
        narrow,
        phase.timing().nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    assert!(
        LcSharedPhase::new(
            timing,
            samples(true)?,
            &interval(["-1/10", "1/10"])?,
            &mut ledger()?
        )
        .is_err()
    );
    assert!(
        LcSharedPhase::new(
            phase.timing().clone(),
            Vec::new(),
            &interval(["0", "0"])?,
            &mut ledger()?
        )
        .is_err()
    );
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = phase.pairs_cover(cycles(), &pi, &mut measured)?;
    let work = measured.usage().work;
    assert_eq!(cover.usage()[1].work - cover.usage()[0].work, work);
    assert!(cover.retained_payload_bytes() > 0);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, nrz_cas::Location::new("phase layers prefix"))?;
        assert_eq!(
            phase.pairs_cover(cycles(), &pi, &mut budget).is_ok(),
            succeeds
        );
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

/// True physical phase constraints contain each complete independent-plus-systematic image.
fn original(initial: Option<[&str; 2]>) -> TestResult<LcRelation> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if let Some(initial) = initial {
        inputs.push(input(Q::InitialPhase, initial)?);
    }
    for (index, (time, phase)) in [("1", ["189/100", "211/100"]), ("2", ["97/25", "103/25"])]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), [time, time])?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    Ok(LcRelation::new(
        "phase measurement epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?)
}

/// Either measurement layer may be absent while retaining the same original physical boxes.
fn question(
    per_sample: bool,
    shared: bool,
    initial: Option<[&str; 2]>,
) -> TestResult<LcSharedPhase> {
    let timing = LcSampleTimes::new(
        original(initial)?,
        vec![number("1")?, number("2")?],
        &mut ledger()?,
    )?;
    let offset = if shared {
        ["-1/10", "1/10"]
    } else {
        ["0", "0"]
    };
    Ok(LcSharedPhase::new(
        timing,
        samples(per_sample)?,
        &interval(offset)?,
        &mut ledger()?,
    )?)
}

/// Unequal sample intervals retain individual identities under the common additive shift.
fn samples(per_sample: bool) -> TestResult<Vec<LcInterval>> {
    let ranges = if per_sample {
        [["199/100", "201/100"], ["199/50", "201/50"]]
    } else {
        [["2", "2"], ["4", "4"]]
    };
    ranges.into_iter().map(interval).collect()
}

/// Phase intervals are deterministic coherent supports, not statistical moments.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Actual local initial phase and shared offset remain separate witness parameters.
fn parameters(rate: &str, initial: &str, offset: &str) -> TestResult<LcPhaseParameters> {
    Ok(LcPhaseParameters {
        physical: LcPairParameters {
            rate: number(rate)?,
            initial_phase: number(initial)?,
            ratio: number("1")?,
            voltage: number("1")?,
            gauge: number("0")?,
        },
        offset: number(offset)?,
    })
}

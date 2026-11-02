// SPDX-License-Identifier: AGPL-3.0-only
//! Both timing layers retain distinct controls and original physical restoration.

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi, point};
use nrz_bindings::{
    LcClockParameters, LcInterval, LcQuantity as Q, LcRelation, LcRelationError, LcSampleTimes,
    LcSharedTiming,
};

/// The joint two-layer family differs from each layer alone on the identical physical relation.
/// ´test:integration:lc-relation-timing-both-layers´
#[test]
fn lc_relation_timing_both_layers() -> TestResult {
    let both = question(true, true)?;
    let cover = both.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("200/103")?, number("200/97")?]);
    let independent = question(true, false)?.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(independent.rate(), &[number("200/101")?, number("200/99")?]);
    let shared = question(false, true)?.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(shared.rate(), &[number("2")?, number("2")?]);
    assert_ne!(cover.rate(), independent.rate());
    assert_ne!(cover.rate(), shared.rate());
    assert!(cover.timing().shares_allocation(&both));
    assert!(cover.relation().shares_allocation(both.relation()));
    assert_eq!(cover.cycles(), cycles());
    assert_eq!(
        cover
            .feasibility_witness()
            .original()
            .values()
            .observations
            .len(),
        2
    );
    for (rate, offset) in [("200/103", "1/25"), ("200/97", "-1/25")] {
        assert_eq!(
            cover.offset_at_rate(&number(rate)?, &mut ledger()?)?,
            [number(offset)?, number(offset)?]
        );
        let witness = cover.witness(
            &LcClockParameters {
                rate: number(rate)?,
                offset: number(offset)?,
                ratio: number("1")?,
                voltage: number("1")?,
            },
            &mut ledger()?,
        )?;
        assert!(witness.timing().shares_allocation(&both));
        assert_eq!(
            witness.original().coordinate(Q::InitialPhase),
            Some(&number("0")?.into())
        );
        both.check_witness(
            witness.original().values().clone(),
            witness.offset().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Marginal time boxes admit tuples forbidden by the retained shared-plus-independent equations.
/// ´test:integration:lc-relation-timing-both-marginal-control´
#[test]
fn lc_relation_timing_both_marginal_control() -> TestResult {
    let both = question(true, true)?;
    let exact = both.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let marginal = LcSampleTimes::new(
        both.relation().clone(),
        both.nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    let independent = marginal.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(independent.rate(), &[number("100/53")?, number("100/47")?]);
    let values = point("21/10", ["20/21", "40/21"])?;
    both.relation()
        .check_witness(values.clone(), &mut ledger()?)?;
    assert!(matches!(
        exact.offset_at_rate(&number("21/10")?, &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    for offset in ["0", "-1/21", "-2/21"] {
        assert!(
            both.check_witness(values.clone(), number(offset)?, &mut ledger()?)
                .is_err()
        );
    }
    Ok(())
}

/// Narrower original supports, malformed inventories and cumulative Work exhaustion stay refusals.
/// ´test:integration:lc-relation-timing-both-refusals-work´
#[test]
fn lc_relation_timing_both_refusals_work() -> TestResult {
    use nrz_bindings::{Budget, Limits, LimitsOverride};
    const FIRST: usize = 0;
    let both = question(true, true)?;
    let errors = errors(true)?;
    let mut inputs = both.relation().inputs().to_vec();
    inputs.push(input(Q::Time(FIRST), ["1", "1"])?);
    let narrow = LcRelation::new(both.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    assert!(
        LcSharedTiming::new(
            narrow,
            both.nominal_times().to_vec(),
            &errors,
            &interval(["-1/10", "1/10"])?,
            &mut ledger()?
        )
        .is_err()
    );
    assert!(
        LcSharedTiming::new(
            both.relation().clone(),
            both.nominal_times().to_vec(),
            &[],
            &interval(["0", "0"])?,
            &mut ledger()?
        )
        .is_err()
    );
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = both.pairs_cover(cycles(), &pi, &mut measured)?;
    let work = measured.usage().work;
    assert_eq!(cover.usage()[1].work - cover.usage()[0].work, work);
    assert!(cover.retained_payload_bytes() > 0);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, nrz_cas::Location::new("both timing layers prefix"))?;
        assert_eq!(
            both.pairs_cover(cycles(), &pi, &mut budget).is_ok(),
            succeeds
        );
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

/// Original total timestamp bounds contain the full image of both separately declared layers.
fn original() -> TestResult<LcRelation> {
    let mut inputs = vec![
        input(Q::Rate, ["1", "3"])?,
        input(Q::InitialPhase, ["0", "0"])?,
    ];
    for (index, (time, phase)) in [(["89/100", "111/100"], "2"), (["47/25", "53/25"], "4")]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    Ok(LcRelation::new(
        "two-layer timing epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?)
}

/// The uncertainty layers can be disabled separately without changing the physical inputs.
fn question(per_sample: bool, shared: bool) -> TestResult<LcSharedTiming> {
    let offset = if shared {
        ["-1/10", "1/10"]
    } else {
        ["0", "0"]
    };
    Ok(LcSharedTiming::new(
        original()?,
        vec![number("1")?, number("2")?],
        &errors(per_sample)?,
        &interval(offset)?,
        &mut ledger()?,
    )?)
}

/// Per-sample widths remain unequal and separately associated with their observations.
fn errors(per_sample: bool) -> TestResult<Vec<LcInterval>> {
    let ranges = if per_sample {
        [["-1/100", "1/100"], ["-1/50", "1/50"]]
    } else {
        [["0", "0"], ["0", "0"]]
    };
    ranges.into_iter().map(interval).collect()
}

/// Exact interval construction uses the unchanged arbitrary-precision resource bounds.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Independent timing relaxation keeps original membership distinct from shared offsets.

mod amplitude;
mod clock_initial;
mod clock_phase;
mod element;
mod energy_only;
mod exact;
mod phase;
mod shared;
mod shared_errors;
mod shared_initial;
mod shared_narrow;
mod support;

use super::support::{TestResult, ledger};
use nrz_bindings::{LcInterval, LcRelationError, LcSampleTimes, LcSharedClock};
use support::{cycles, number, original, pi, point, sample_times};

/// Unequal signed error boxes use all four exact rational rate/error corners.
/// ´test:integration:lc-relation-timing-outer-corners´
#[test]
fn lc_relation_timing_outer_corners() -> TestResult {
    let timing = sample_times()?;
    assert_eq!(timing.rate_support(), &[number("1")?, number("3")?]);
    assert_eq!(
        timing.errors(),
        &[
            [number("-1/10")?, number("1/10")?],
            [number("-1/5")?, number("3/10")?],
        ]
    );
    let cover = timing.outer_pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(cover.timing().shares_allocation(&timing));
    assert_eq!(
        cover.timing_advances(),
        &[
            [number("-3/10")?, number("3/10")?],
            [number("-3/5")?, number("9/10")?],
        ]
    );
    assert_eq!(cover.rate(), &[number("17/10")?, number("23/10")?]);
    for (nominal, expected) in [
        (["0", "0"], [["9/10", "33/10"], ["9/5", "69/10"]]),
        (["2", "3"], [["-33/10", "-9/10"], ["-18/5", "-7/10"]]),
    ] {
        let timing = LcSampleTimes::new(
            original()?,
            vec![number(nominal[0])?, number(nominal[1])?],
            &mut ledger()?,
        )?;
        let cover = timing.outer_pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        for (actual, expected) in cover.timing_advances().iter().zip(expected) {
            assert_eq!(actual, &[number(expected[0])?, number(expected[1])?]);
        }
    }
    Ok(())
}

/// A relaxed boundary rate is not a witness until original equations and time boxes pass.
/// ´test:integration:lc-relation-timing-outer-witness-boundary´
#[test]
fn lc_relation_timing_outer_witness_boundary() -> TestResult {
    let timing = sample_times()?;
    let cover = timing.outer_pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let zero = number("0")?;
    assert_eq!(
        cover.phase_at_rate(&number("17/10")?, &mut ledger()?)?,
        [zero.clone(), zero]
    );
    // Exact phase equations force times 20/17 and 40/17, outside the original boxes.
    assert!(matches!(
        cover.check_witness(point("17/10", ["20/17", "40/17"])?, &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    let witness = cover.check_witness(point("2", ["1", "2"])?, &mut ledger()?)?;
    assert!(witness.relation().shares_allocation(timing.relation()));
    let mut wrong_equation = point("2", ["1", "2"])?;
    wrong_equation.observations[0].time = number("11/10")?.into();
    assert!(matches!(
        cover.check_witness(wrong_equation, &mut ledger()?),
        Err(LcRelationError::Equation)
    ));
    Ok(())
}

/// A common offset cancels from phase differences; independent errors admit distinct offsets.
/// ´test:integration:lc-relation-timing-outer-shared-control´
#[test]
fn lc_relation_timing_outer_shared_control() -> TestResult {
    let timing = sample_times()?;
    let outer = timing.outer_pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let offset = LcInterval::new(
        Some(number("-1/10")?.into()),
        Some(number("1/10")?.into()),
        &mut ledger()?,
    )?;
    let clock = LcSharedClock::new(
        timing.relation().clone(),
        timing.nominal_times().to_vec(),
        &offset,
        &mut ledger()?,
    )?;
    let shared = clock.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(shared.rate(), &[number("2")?, number("2")?]);
    assert_ne!(outer.rate(), shared.rate());
    let independent = point("41/20", ["40/41", "80/41"])?;
    outer.check_witness(independent.clone(), &mut ledger()?)?;
    for offset in ["0", "-1/41", "-2/41"] {
        assert!(
            clock
                .check_witness(independent.clone(), number(offset)?, &mut ledger()?)
                .is_err()
        );
    }
    Ok(())
}

/// Intersections retain repeated inputs, while missing rates and unproved branches refuse.
/// ´test:integration:lc-relation-timing-outer-refusals´
#[test]
fn lc_relation_timing_outer_refusals() -> TestResult {
    use nrz_bindings::{LcQuantity as Q, LcRelation};
    const FIRST: usize = 0;
    let relation = original()?;
    let mut inputs = relation.inputs().to_vec();
    inputs.push(support::input(Q::Time(FIRST), ["1", "1"])?);
    let refined = LcRelation::new(relation.epoch().into(), 2, inputs, &mut ledger()?)?;
    let timing = LcSampleTimes::new(refined, vec![number("1")?, number("2")?], &mut ledger()?)?;
    assert_eq!(timing.errors()[FIRST], [number("0")?, number("0")?]);
    let cover = timing.outer_pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
    assert!(LcSampleTimes::new(relation.clone(), Vec::new(), &mut ledger()?).is_err());
    let mut proposal = cycles();
    proposal[FIRST] = number("1/2")?;
    assert!(
        sample_times()?
            .outer_pairs_cover(proposal, &pi()?, &mut ledger()?)
            .is_err()
    );
    let mut inputs = relation.inputs().to_vec();
    inputs.retain(|input| input.quantity() != Q::Rate);
    let missing = LcRelation::new(relation.epoch().into(), 2, inputs, &mut ledger()?)?;
    assert!(LcSampleTimes::new(missing, vec![number("1")?, number("2")?], &mut ledger()?).is_err());
    Ok(())
}

/// Work boundaries retain the original question and never reset a consumed caller prefix.
/// ´test:integration:lc-relation-timing-outer-work´
#[test]
fn lc_relation_timing_outer_work() -> TestResult {
    use nrz_bindings::{Budget, Limits, LimitsOverride};
    use nrz_cas::Location;
    let timing = sample_times()?;
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = timing.outer_pairs_cover(cycles(), &pi, &mut measured)?;
    let work = measured.usage().work;
    assert!(work > 0);
    assert_eq!(cover.usage()[1].work - cover.usage()[0].work, work);
    assert!(cover.retained_payload_bytes() > 0);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, Location::new("independent timing prefix"))?;
        assert_eq!(
            timing.outer_pairs_cover(cycles(), &pi, &mut budget).is_ok(),
            succeeds
        );
        assert!(budget.usage().work >= 17);
        assert!(cover.timing().shares_allocation(&timing));
    }
    Ok(())
}

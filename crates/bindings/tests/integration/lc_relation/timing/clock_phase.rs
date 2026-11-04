// SPDX-License-Identifier: AGPL-3.0-only
//! One original question retains the correlation of shared clock and shared phase errors.

mod cycle_clip;
mod narrow;
mod support;

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{LcPhaseGauge, LcQuantity as Q};
use support::{parameters, question, work_boundary};

/// Both shared errors permit a continuous joint family that each error alone excludes.
/// ´test:integration:lc-relation-clock-phase-combined-controls´
#[test]
fn lc_relation_clock_phase_combined_controls() -> TestResult {
    support::separate_layer_controls()?;
    let source = question([true, true, true, false])?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for (clock_offset, phase_offset) in [("-1/10", "-1/5"), ("0", "0"), ("1/10", "1/5")] {
        assert_eq!(
            cover.clock_offset_at_phases(
                &number("2")?,
                &number("0")?,
                &number(phase_offset)?,
                &mut ledger()?
            )?,
            [number(clock_offset)?, number(clock_offset)?]
        );
        let point = cover.witness(
            &parameters("2", "0", clock_offset, phase_offset, "0")?,
            &mut ledger()?,
        )?;
        source.check_witness(
            point.original().original().values().clone(),
            number(clock_offset)?,
            number(phase_offset)?,
            &mut ledger()?,
        )?;
    }
    assert!(
        cover
            .witness(&parameters("2", "0", "-1/10", "1/5", "0")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Original marginal total boxes admit independently crossed timestamps and phase offsets.
/// ´test:integration:lc-relation-clock-phase-marginal-control´
#[test]
fn lc_relation_clock_phase_marginal_control() -> TestResult {
    let source = question([true, true, true, false])?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let original = cover.witness(&parameters("2", "0", "0", "0", "0")?, &mut ledger()?)?;
    let mut crossed = original.original().original().values().clone();
    for (observation, (time, phase)) in crossed
        .observations
        .iter_mut()
        .zip([("11/10", "11/5"), ("19/10", "19/5")])
    {
        observation.time = number(time)?.into();
        observation.phase = Some(number(phase)?.into());
    }
    source
        .relation()
        .check_witness(crossed.clone(), &mut ledger()?)?;
    assert!(
        source
            .check_witness(
                crossed.clone(),
                number("1/10")?,
                number("1/5")?,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(
        source
            .check_witness(crossed, number("-1/10")?, number("-1/5")?, &mut ledger()?)
            .is_err()
    );
    support::independent_phase_control(&source)
}

/// Unknown physical initial phase retains both error fibres and every common integer gauge.
/// ´test:integration:lc-relation-clock-phase-initial-fibres´
#[test]
fn lc_relation_clock_phase_initial_fibres() -> TestResult {
    let source = question([true, true, false, false])?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-2/5")?, number("2/5")?]
    );
    for (alpha, clock_offset, phase_offset) in [
        ("-2/5", "1/10", "-1/5"),
        ("0", "0", "0"),
        ("2/5", "-1/10", "1/5"),
    ] {
        for gauge in ["-2", "0", "3"] {
            let point = cover.witness(
                &parameters("2", alpha, clock_offset, phase_offset, gauge)?,
                &mut ledger()?,
            )?;
            source.check_witness(
                point.original().original().values().clone(),
                number(clock_offset)?,
                number(phase_offset)?,
                &mut ledger()?,
            )?;
        }
    }
    assert_eq!(
        cover.offset_at_phase(&number("2")?, &number("2/5")?, &mut ledger()?)?,
        [number("1/5")?, number("1/5")?]
    );
    assert_eq!(
        cover.clock_offset_at_phases(
            &number("2")?,
            &number("2/5")?,
            &number("1/5")?,
            &mut ledger()?
        )?,
        [number("-1/10")?, number("-1/10")?]
    );
    assert!(
        cover
            .witness(&parameters("2", "2/5", "1/10", "1/5", "0")?, &mut ledger()?)
            .is_err()
    );
    // Scalar phase queries must not publish the effective alpha-minus-offset coordinate.
    assert!(cover.bounds(Q::InitialPhase, &mut ledger()?).is_err());
    Ok(())
}

/// Independent measured phase intervals and both shared errors preserve exact original branch evidence.
/// ´test:integration:lc-relation-clock-phase-sample-intervals´
#[test]
fn lc_relation_clock_phase_sample_intervals() -> TestResult {
    let source = question([true, true, true, true])?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("99/50")?, number("101/50")?]);
    for (rate, phase_offset) in [("99/50", "-3/100"), ("101/50", "3/100")] {
        let point = cover.witness(
            &parameters(rate, "0", "0", phase_offset, "0")?,
            &mut ledger()?,
        )?;
        source.check_witness(
            point.original().original().values().clone(),
            number("0")?,
            number(phase_offset)?,
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Incomplete cycles, invalid shared-error choices and cumulative Work refuse without narrowing.
/// ´test:integration:lc-relation-clock-phase-refusals-work´
#[test]
fn lc_relation_clock_phase_refusals_work() -> TestResult {
    let source = question([true, true, true, false])?;
    let pi = pi()?;
    assert!(
        source
            .pairs_cover(vec![number("0")?, number("1")?], &pi, &mut ledger()?)
            .is_err()
    );
    let mut measured = ledger()?;
    let cover = source.pairs_cover(cycles(), &pi, &mut measured)?;
    assert_eq!(
        cover.usage()[1].work - cover.usage()[0].work,
        measured.usage().work
    );
    work_boundary(|budget| source.pairs_cover(cycles(), &pi, budget))?;
    let point = parameters("2", "0", "1/10", "1/5", "0")?;
    work_boundary(|budget| cover.witness(&point, budget))?;
    work_boundary(|budget| {
        cover.offset_at_phase(&point.physical.rate, &point.physical.initial_phase, budget)
    })?;
    work_boundary(|budget| {
        cover.clock_offset_at_phases(
            &point.physical.rate,
            &point.physical.initial_phase,
            &point.offset,
            budget,
        )
    })?;
    assert!(
        cover
            .witness(&parameters("2", "0", "1/5", "2/5", "0")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

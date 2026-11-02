// SPDX-License-Identifier: AGPL-3.0-only
//! A single common clock offset cannot be replaced by independent timestamp boxes.

mod refusals;

use super::super::support::{TestResult, input, integer, ledger, number, pi_certificate};
use nrz_bindings::{
    LcClockCover, LcClockParameters, LcInterval, LcQuantity as Q, LcRelation, LcSharedClock,
};
use nrz_matrix::LcClockProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// Independent original boxes deliberately admit more tuples than the coupled clock question.
pub(super) fn relation() -> TestResult<LcRelation> {
    relation_phases(["2", "4"])
}

/// Exact phase centers vary independently of the clock elimination implementation.
fn relation_phases(phases: [&str; 2]) -> TestResult<LcRelation> {
    let mut inputs = vec![
        input(Q::Rate, ["19/10", "21/10"])?,
        input(Q::InitialPhase, ["0", "0"])?,
    ];
    for (index, (times, phase)) in [["9/10", "11/10"], ["19/10", "21/10"]]
        .into_iter()
        .zip(phases)
        .enumerate()
    {
        inputs.push(input(Q::Time(index), times)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    Ok(LcRelation::new(
        "shared clock epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?)
}

/// The same bounded duration offset is shared by all nominal timestamps.
pub(super) fn clock() -> TestResult<LcSharedClock> {
    let offset = LcInterval::new(
        Some(number("-1/10")?.into()),
        Some(number("1/10")?.into()),
        &mut ledger()?,
    )?;
    Ok(LcSharedClock::new(
        relation()?,
        vec![number("1")?, number("2")?],
        &offset,
        &mut ledger()?,
    )?)
}

/// Both formats retain the exact rate/product result and the single correlated offset fibre.
/// ´test:integration:lc-relation-shared-clock´
#[test]
fn lc_relation_shared_clock() -> TestResult {
    let clock = clock()?;
    let cover = clock.pairs_cover(
        vec![integer(0)?, integer(0)?],
        &pi_certificate()?,
        &mut ledger()?,
    )?;
    assert!(cover.clock().shares_allocation(&clock));
    assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
    assert_eq!(
        cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
        [number("0")?, number("0")?]
    );
    assert_eq!(cover.cycles(), &[number("0")?, number("0")?]);
    assert_eq!(cover.relation().epoch(), "shared clock epoch");
    assert_eq!(
        cover.clock().offset_descriptor(),
        &Q::NaturalTime.descriptor()?
    );
    assert_eq!(
        cover
            .feasibility_witness()
            .original()
            .values()
            .observations
            .len(),
        2
    );
    render::<f32>(&cover)?;
    render::<f64>(&cover)?;
    let point = cover.witness(
        &LcClockParameters {
            rate: number("2")?,
            offset: number("0")?,
            ratio: number("1")?,
            voltage: number("1")?,
        },
        &mut ledger()?,
    )?;
    assert!(point.clock().shares_allocation(&clock));
    assert_eq!(
        point.original().coordinate(Q::InitialPhase),
        Some(&number("0")?.into())
    );
    Ok(())
}

/// Expected exact fractions follow independently from the phase difference and shared clock cancellation.
fn render<P: Scalar>(cover: &LcClockCover) -> TestResult {
    let context = ArithmeticContext::new()?;
    let native = LcClockProjection::<P>::new(
        cover.clone(),
        &[Q::Rate, Q::Product, Q::NaturalTime, Q::InitialPhase],
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    assert!(native.cover().clock().shares_allocation(cover.clock()));
    for (quantity, expected) in native.quantities().iter().zip(["2", "1/4", "1/2", "0"]) {
        for endpoint in quantity.endpoints() {
            super::capacitance::contains(
                endpoint.ok_or("finite clock projection")?,
                &number(expected)?,
            )?;
        }
    }
    Ok(())
}

/// Independent timing boxes accept this physical tuple; the shared-clock checker must reject it.
/// ´test:integration:lc-relation-shared-clock-independent-box-control´
#[test]
fn lc_relation_shared_clock_independent_box_control() -> TestResult {
    let clock = clock()?;
    let cover = clock.pairs_cover(
        vec![integer(0)?, integer(0)?],
        &pi_certificate()?,
        &mut ledger()?,
    )?;
    let mut values = cover.feasibility_witness().original().values().clone();
    values.rate = number("41/20")?.into();
    values.inductance = number("20/41")?.into();
    values.capacitance = number("20/41")?.into();
    values.voltage = number("1")?.into();
    values.current = number("1")?.into();
    values.energy = number("10/41")?.into();
    for (observation, time) in values.observations.iter_mut().zip(["40/41", "80/41"]) {
        observation.time = number(time)?.into();
    }
    assert!(
        clock
            .relation()
            .check_witness(values.clone(), &mut ledger()?)
            .is_ok()
    );
    for offset in ["0", "-1/41", "-2/41"] {
        assert!(
            clock
                .check_witness(values.clone(), number(offset)?, &mut ledger()?)
                .is_err()
        );
    }
    assert!(
        cover
            .offset_at_rate(&number("41/20")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Nonzero offsets of either sign are recovered on the original physical clock, not the effective lift.
/// ´test:integration:lc-relation-shared-clock-nonzero-offsets´
#[test]
fn lc_relation_shared_clock_nonzero_offsets() -> TestResult {
    for (phases, expected, times) in [
        (["21/10", "41/10"], "1/20", ["21/20", "41/20"]),
        (["19/10", "39/10"], "-1/20", ["19/20", "39/20"]),
    ] {
        let offset = LcInterval::new(
            Some(number("-1/10")?.into()),
            Some(number("1/10")?.into()),
            &mut ledger()?,
        )?;
        let clock = LcSharedClock::new(
            relation_phases(phases)?,
            vec![number("1")?, number("2")?],
            &offset,
            &mut ledger()?,
        )?;
        let cover = clock.pairs_cover(
            vec![integer(0)?, integer(0)?],
            &pi_certificate()?,
            &mut ledger()?,
        )?;
        assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
        assert_eq!(
            cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
            [number(expected)?, number(expected)?]
        );
        let point = cover.witness(
            &LcClockParameters {
                rate: number("2")?,
                offset: number(expected)?,
                ratio: number("3")?,
                voltage: number("2")?,
            },
            &mut ledger()?,
        )?;
        for (observation, time) in point.original().values().observations.iter().zip(times) {
            assert_eq!(observation.time.as_rational(), Some(&number(time)?));
        }
        assert_eq!(
            cover.free_directions(Q::Inductance)?,
            &[nrz_bindings::LcFreeDirection::ElementScale]
        );
    }
    Ok(())
}

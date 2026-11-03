// SPDX-License-Identifier: AGPL-3.0-only
//! Energy-only original questions retain free physical scales and exact timing fibres.

mod calibrated;

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    Budget, LcCertainty, LcDeterminacy, LcEnergyOnlyCover, LcEnergyOnlyParameters, LcFreeDirection,
    LcInterval, LcPhaseGauge, LcQuantity as Q, LcRelation, LcSampleTimes, Limits, LimitsOverride,
};

/// Independent energy restrictions intersect at [1/4,3/10] without any amplitude channel.
fn question(uncertain: bool, known: bool) -> TestResult<LcSampleTimes> {
    let original = super::support::original()?;
    let mut inputs = original.inputs().to_vec();
    if !uncertain {
        inputs.retain(|value| !matches!(value.quantity(), Q::Time(_)));
        for (index, time) in ["1", "2"].into_iter().enumerate() {
            inputs.push(input(Q::Time(index), [time, time])?);
        }
    }
    if !known {
        inputs.retain(|value| value.quantity() != Q::InitialPhase);
    }
    inputs.push(input(Q::Energy, ["1/5", "3/10"])?);
    inputs.push(input(Q::Energy, ["1/4", "2/5"])?);
    let relation = LcRelation::new(original.epoch().into(), 2, inputs, &mut ledger()?)?;
    Ok(LcSampleTimes::new(
        relation,
        vec![number("1")?, number("2")?],
        &mut ledger()?,
    )?)
}

/// Every positive voltage names a member of the one free element/amplitude direction.
fn parameters(energy: &str, voltage: &str) -> TestResult<LcEnergyOnlyParameters> {
    Ok(LcEnergyOnlyParameters {
        rate: number("2")?,
        initial_phase: number("0")?,
        gauge: number("0")?,
        energy: number(energy)?,
        voltage: number(voltage)?,
    })
}

/// Exact endpoint comparison does not evaluate the production scale formulas.
fn bounds(interval: &LcInterval, expected: [&str; 2]) -> TestResult {
    assert_eq!(interval.lower(), Some(&number(expected[0])?.into()));
    assert_eq!(interval.upper(), Some(&number(expected[1])?.into()));
    Ok(())
}

/// Energy stays finite while both physical scale families remain explicitly positive and unbounded.
/// ´test:integration:lc-relation-energy-without-amplitude´
#[test]
fn lc_relation_energy_without_amplitude() -> TestResult {
    let timing = question(false, true)?;
    let cover = timing.energy_only_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(cover.timing().shares_allocation(&timing));
    assert_eq!(cover.energy(), &[number("1/4")?, number("3/10")?]);
    assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
    assert_eq!(cover.certainty(), LcCertainty::Certified);
    for (quantity, expected) in [
        (Q::Energy, ["1/4", "3/10"]),
        (Q::Product, ["1/4", "1/4"]),
        (Q::NaturalTime, ["1/2", "1/2"]),
    ] {
        bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
        assert_eq!(
            cover.determinacy(quantity)?,
            LcDeterminacy::DeterminedFinite
        );
        assert_eq!(cover.free_directions(quantity)?, [] as [LcFreeDirection; 0]);
    }
    free_scales(&cover)?;
    physical_oracles(&cover)?;
    Ok(())
}

/// A free positive voltage varies every unbounded scale without admitting zero.
fn free_scales(cover: &LcEnergyOnlyCover) -> TestResult {
    for quantity in [
        Q::Inductance,
        Q::Capacitance,
        Q::AmplitudeRatio,
        Q::VoltageAmplitude,
        Q::CurrentAmplitude,
    ] {
        assert_eq!(cover.determinacy(quantity)?, LcDeterminacy::Underdetermined);
        assert_eq!(
            cover.free_directions(quantity)?,
            &[LcFreeDirection::ElementScale]
        );
        let interval = cover.bounds(quantity, &mut ledger()?)?;
        assert_eq!(interval.lower(), Some(&number("0")?.into()));
        assert_eq!(interval.upper(), None);
    }
    Ok(())
}

/// Independent literal LC solutions check both energy endpoints and nonunit voltages.
fn physical_oracles(cover: &LcEnergyOnlyCover) -> TestResult {
    for (energy, voltage, inductance, capacitance, current) in [
        ("1/4", "1", "1/2", "1/2", "1"),
        ("3/10", "1", "5/12", "3/5", "6/5"),
        ("1/4", "2", "2", "1/8", "1/2"),
        ("1/4", "1/2", "1/8", "2", "2"),
        ("11/40", "1", "5/11", "11/20", "11/10"),
    ] {
        let witness = cover.witness(&parameters(energy, voltage)?, &mut ledger()?)?;
        for (quantity, expected) in [
            (Q::Inductance, inductance),
            (Q::Capacitance, capacitance),
            (Q::CurrentAmplitude, current),
        ] {
            assert_eq!(
                witness.coordinate(quantity),
                Some(&number(expected)?.into())
            );
        }
        assert!(witness.relation().shares_allocation(cover.relation()));
        cover
            .relation()
            .check_witness(witness.values().clone(), &mut ledger()?)?;
    }
    Ok(())
}

/// Independent time uncertainty and unknown initial phase retain their original conditional fibres.
/// ´test:integration:lc-relation-energy-without-amplitude-timing´
#[test]
fn lc_relation_energy_without_amplitude_timing() -> TestResult {
    for known in [false, true] {
        let timing = question(true, known)?;
        let cover = timing.energy_only_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_eq!(cover.cycles(), cycles());
        assert_eq!(
            cover.phase_gauge(),
            if known {
                LcPhaseGauge::Fixed
            } else {
                LcPhaseGauge::AllIntegers
            }
        );
        if known {
            assert_eq!(cover.rate(), &[number("20/11")?, number("20/9")?]);
        }
        for rate in cover.rate() {
            for alpha in cover.phase_at_rate(rate, &mut ledger()?)? {
                let mut parameters = parameters("11/40", "3/2")?;
                parameters.rate = rate.clone();
                parameters.initial_phase = alpha;
                if !known {
                    parameters.gauge = number("-2")?;
                }
                let witness = cover.witness(&parameters, &mut ledger()?)?;
                timing
                    .relation()
                    .check_witness(witness.values().clone(), &mut ledger()?)?;
                assert_eq!(
                    witness.values().observations.len(),
                    timing.time_boxes().len()
                );
            }
        }
    }
    Ok(())
}

/// Original energy restrictions and all old equations reject crossed scale coordinates.
/// ´test:integration:lc-relation-energy-without-amplitude-original-inputs´
#[test]
fn lc_relation_energy_without_amplitude_original_inputs() -> TestResult {
    let timing = question(false, true)?;
    let cover = timing.energy_only_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.relation().inputs(), timing.relation().inputs());
    assert!(
        cover
            .feasibility_witness()
            .relation()
            .shares_allocation(timing.relation())
    );
    for (energy, voltage) in [
        ("1/5", "1"),
        ("2/5", "1"),
        ("0", "1"),
        ("1/4", "0"),
        ("1/4", "-1"),
    ] {
        assert!(
            cover
                .witness(&parameters(energy, voltage)?, &mut ledger()?)
                .is_err()
        );
    }
    let point = cover.witness(&parameters("1/4", "1")?, &mut ledger()?)?;
    let mut crossed = point.values().clone();
    crossed.energy = number("3/10")?.into();
    assert!(
        timing
            .relation()
            .check_witness(crossed, &mut ledger()?)
            .is_err()
    );
    for extra in [
        input(Q::Energy, ["1/2", "3/5"])?,
        input(Q::VoltageAmplitude, ["1", "1"])?,
        input(Q::Capacitance, ["1/2", "1/2"])?,
    ] {
        let mut inputs = timing.relation().inputs().to_vec();
        inputs.push(extra);
        let relation =
            LcRelation::new(timing.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
        let timing = LcSampleTimes::new(relation, timing.nominal_times().to_vec(), &mut ledger()?)?;
        assert!(
            timing
                .energy_only_cover(cycles(), &pi()?, &mut ledger()?)
                .is_err()
        );
    }
    assert!(
        timing
            .energy_only_cover(vec![number("0")?, number("1")?], &pi()?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Exact cumulative Work boundaries include original restoration and preserve the old question.
/// ´test:integration:lc-relation-energy-without-amplitude-work´
#[test]
fn lc_relation_energy_without_amplitude_work() -> TestResult {
    let timing = question(false, true)?;
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = timing.energy_only_cover(cycles(), &pi, &mut measured)?;
    assert_eq!(
        cover.usage()[1].work - cover.usage()[0].work,
        measured.usage().work
    );
    boundary(measured.usage().work, |budget| {
        timing.energy_only_cover(cycles(), &pi, budget).is_ok()
    })?;
    let parameters = parameters("11/40", "3/2")?;
    let mut measured = ledger()?;
    cover.witness(&parameters, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        cover.witness(&parameters, budget).is_ok()
    })?;
    for quantity in [Q::Energy, Q::CurrentAmplitude] {
        let mut measured = ledger()?;
        cover.bounds(quantity, &mut measured)?;
        boundary(measured.usage().work, |budget| {
            cover.bounds(quantity, budget).is_ok()
        })?;
    }
    Ok(())
}

/// Exact N/N-1 boundaries include the caller's already consumed prefix without raising ceilings.
fn boundary(work: u64, operation: impl Fn(&mut Budget) -> bool) -> TestResult {
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, nrz_cas::Location::new("energy-only prefix"))?;
        assert_eq!(operation(&mut budget), success);
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

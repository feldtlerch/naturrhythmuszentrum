// SPDX-License-Identifier: AGPL-3.0-only
//! Energy calibration retains its original readings while physical voltage remains unmeasured.

use super::{TestResult, boundary, bounds, cycles, input, ledger, number, parameters, pi};
use nrz_bindings::{
    LcCertainty, LcConsistency, LcDeterminacy, LcEnergyOnlyCalibrationCover, LcEnergyOnlySamples,
    LcFreeDirection, LcInterval, LcQuantity as Q, LcRelation, LcSampleTimes,
};

/// Optional original physical energy bounds are distinct from measured reading supports.
fn timing(uncertain: bool, known: bool, physical: Option<[&str; 2]>) -> TestResult<LcSampleTimes> {
    let old = super::question(uncertain, known)?;
    let mut inputs = old.relation().inputs().to_vec();
    inputs.retain(|input| input.quantity() != Q::Energy);
    if let Some(range) = physical {
        inputs.push(input(Q::Energy, range)?);
    }
    let relation = LcRelation::new(old.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    Ok(LcSampleTimes::new(
        relation,
        old.nominal_times().to_vec(),
        &mut ledger()?,
    )?)
}

/// Positive coherent reading or calibration supports remain exact.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Unequal independent readings intersect before one common calibration is applied.
fn samples(independent: bool) -> TestResult<Vec<LcInterval>> {
    if independent {
        Ok(vec![interval(["1/5", "3/10"])?, interval(["1/4", "2/5"])?])
    } else {
        Ok(vec![interval(["1/4", "1/4"])?, interval(["1/4", "1/4"])?])
    }
}

/// Both layers and each separate layer have independently specified exact physical images.
/// ´test:integration:lc-relation-energy-only-calibration-layers´
#[test]
fn lc_relation_energy_only_calibration_layers() -> TestResult {
    let pi = pi()?;
    for (independent, shared, expected) in [
        (true, true, ["9/40", "33/100"]),
        (true, false, ["1/4", "3/10"]),
        (false, true, ["9/40", "11/40"]),
        (false, false, ["1/4", "1/4"]),
    ] {
        let timing = timing(false, true, None)?;
        let gain = if shared {
            ["9/10", "11/10"]
        } else {
            ["1", "1"]
        };
        let readings = samples(independent)?;
        let energy = LcEnergyOnlySamples::new(
            timing.clone(),
            readings.clone(),
            &interval(gain)?,
            &mut ledger()?,
        )?;
        assert!(energy.timing().shares_allocation(&timing));
        assert_eq!(energy.samples(), readings);
        assert_eq!(energy.calibration(), &[number(gain[0])?, number(gain[1])?]);
        let cover = energy.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        assert!(cover.energy().shares_allocation(&energy));
        assert!(cover.relation().shares_allocation(timing.relation()));
        bounds(&cover.bounds(Q::Energy, &mut ledger()?)?, expected)?;
        assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
        assert_eq!(cover.certainty(), LcCertainty::Certified);
        assert_eq!(cover.consistency(), LcConsistency::Feasible);
        assert_eq!(
            cover.determinacy(Q::Energy)?,
            LcDeterminacy::DeterminedFinite
        );
        assert_eq!(
            cover.free_directions(Q::Energy)?,
            [] as [LcFreeDirection; 0]
        );
        free(&cover)?;
        let point = cover.feasibility_witness();
        assert!(point.energy().shares_allocation(&energy));
        assert!(
            point
                .original()
                .relation()
                .shares_allocation(timing.relation())
        );
        energy.check_witness(
            point.original().values().clone(),
            point.calibration().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Energy calibration supplies no amplitude measurement or finite element scale.
fn free(cover: &LcEnergyOnlyCalibrationCover) -> TestResult {
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
        let range = cover.bounds(quantity, &mut ledger()?)?;
        assert_eq!(range.lower(), Some(&number("0")?.into()));
        assert_eq!(range.upper(), None);
    }
    Ok(())
}

/// Original physical energy bounds retain conditional reading and calibration fibres.
/// ´test:integration:lc-relation-energy-only-calibration-fibres´
#[test]
fn lc_relation_energy_only_calibration_fibres() -> TestResult {
    let timing = super::question(false, true)?;
    let energy = LcEnergyOnlySamples::new(
        timing.clone(),
        samples(true)?,
        &interval(["9/10", "11/10"])?,
        &mut ledger()?,
    )?;
    let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.relation().inputs(), timing.relation().inputs());
    bounds(&cover.bounds(Q::Energy, &mut ledger()?)?, ["1/4", "3/10"])?;
    for (gain, expected) in [
        ("9/10", ["5/18", "3/10"]),
        ("1", ["1/4", "3/10"]),
        ("11/10", ["1/4", "3/11"]),
    ] {
        let reading = cover.reading_at_calibration(&number(gain)?, &mut ledger()?)?;
        assert_eq!(reading, [number(expected[0])?, number(expected[1])?]);
        for reading in reading {
            let value = reading.mul(&number(gain)?, &mut ledger()?)?;
            let mut point = parameters("1/4", "3/2")?;
            point.energy = value;
            cover.witness(&point, number(gain)?, &mut ledger()?)?;
        }
    }
    for (value, expected) in [("1/4", ["9/10", "1"]), ("3/10", ["1", "11/10"])] {
        assert_eq!(
            cover.calibration_at_energy(&number(value)?, &mut ledger()?)?,
            [number(expected[0])?, number(expected[1])?]
        );
    }
    assert!(
        cover
            .calibration_at_energy(&number("9/40")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(&parameters("1/4", "1")?, number("11/10")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(&parameters("3/10", "1")?, number("9/10")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .reading_at_calibration(&number("2")?, &mut ledger()?)
            .is_err()
    );
    let mut wrong = cover.feasibility_witness().original().values().clone();
    wrong.energy = number("1/4")?.into();
    wrong.current = number("7")?.into();
    assert!(
        energy
            .check_witness(wrong, number("1")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Independent literal physical tuples cover both energy extrema and distinct free voltages.
/// ´test:integration:lc-relation-energy-only-calibration-physics´
#[test]
fn lc_relation_energy_only_calibration_physics() -> TestResult {
    let energy = LcEnergyOnlySamples::new(
        timing(false, true, None)?,
        samples(true)?,
        &interval(["9/10", "11/10"])?,
        &mut ledger()?,
    )?;
    let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for (value, gain, voltage, inductance, capacitance, current) in [
        ("9/40", "9/10", "1", "5/9", "9/20", "9/10"),
        ("33/100", "11/10", "1", "25/66", "33/50", "33/25"),
        ("9/40", "9/10", "2", "20/9", "9/80", "9/20"),
        ("9/40", "9/10", "1/2", "5/36", "9/5", "9/5"),
        ("1/4", "1", "3/2", "9/8", "2/9", "2/3"),
    ] {
        let witness = cover.witness(&parameters(value, voltage)?, number(gain)?, &mut ledger()?)?;
        for (quantity, expected) in [
            (Q::Inductance, inductance),
            (Q::Capacitance, capacitance),
            (Q::CurrentAmplitude, current),
        ] {
            assert_eq!(
                witness.original().coordinate(quantity),
                Some(&number(expected)?.into())
            );
        }
        energy.check_witness(
            witness.original().values().clone(),
            number(gain)?,
            &mut ledger()?,
        )?;
    }
    timing_fibres()?;
    Ok(())
}

/// Small dyadic timestamp boxes retain unknown physical phase and a nonzero common gauge.
fn timing_fibres() -> TestResult {
    let pi = pi()?;
    for known in [true, false] {
        let old = timing(true, known, Some(["1/4", "3/10"]))?;
        let mut inputs = old.relation().inputs().to_vec();
        inputs.retain(|input| !matches!(input.quantity(), Q::Time(_)));
        for (index, range) in [["3/4", "5/4"], ["7/4", "9/4"]].into_iter().enumerate() {
            inputs.push(input(Q::Time(index), range)?);
        }
        let relation = LcRelation::new(old.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
        let timing = LcSampleTimes::new(relation, old.nominal_times().to_vec(), &mut ledger()?)?;
        let energy = LcEnergyOnlySamples::new(
            timing,
            samples(true)?,
            &interval(["9/10", "11/10"])?,
            &mut ledger()?,
        )?;
        let cover = energy
            .pairs_cover(cycles(), &pi, &mut ledger()?)
            .map_err(|error| format!("timing energy cover, known={known}: {error}"))?;
        for rate in cover.rate() {
            for phase in cover.phase_at_rate(rate, &mut ledger()?)? {
                let mut point = parameters("1/4", "1")?;
                point.rate = rate.clone();
                point.initial_phase = phase;
                if !known {
                    point.gauge = number("-1")?;
                }
                let witness = cover
                    .witness(&point, number("1")?, &mut ledger()?)
                    .map_err(|error| {
                        format!(
                            "timing energy witness, known={known}, parameters={point:?}: {error}"
                        )
                    })?;
                energy
                    .check_witness(
                        witness.original().values().clone(),
                        number("1")?,
                        &mut ledger()?,
                    )
                    .map_err(|error| format!("timing original recheck, known={known}: {error}"))?;
            }
        }
    }
    Ok(())
}

/// Different gains per sample can satisfy total boxes but cannot satisfy a single original calibration.
/// ´test:integration:lc-relation-energy-only-calibration-shared-control´
#[test]
fn lc_relation_energy_only_calibration_shared_control() -> TestResult {
    let timing = timing(false, true, Some(["2", "2"]))?;
    let physical = timing.energy_only_cover(cycles(), &pi()?, &mut ledger()?)?;
    physical.witness(&parameters("2", "1")?, &mut ledger()?)?;
    let value = number("2")?;
    assert_eq!(value.div(&number("1")?, &mut ledger()?)?, number("2")?);
    assert_eq!(value.div(&number("2")?, &mut ledger()?)?, number("1")?);
    assert!(
        LcEnergyOnlySamples::new(
            timing,
            vec![interval(["1", "1"])?, interval(["2", "2"])?],
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Construction, reconstruction and each conditional fibre have exact cumulative Work boundaries.
/// ´test:integration:lc-relation-energy-only-calibration-work´
#[test]
fn lc_relation_energy_only_calibration_work() -> TestResult {
    let timing = timing(false, true, None)?;
    let samples = samples(true)?;
    let gain = interval(["9/10", "11/10"])?;
    let mut measured = ledger()?;
    let energy = LcEnergyOnlySamples::new(timing.clone(), samples.clone(), &gain, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        LcEnergyOnlySamples::new(timing.clone(), samples.clone(), &gain, budget).is_ok()
    })?;
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = energy.pairs_cover(cycles(), &pi, &mut measured)?;
    assert_eq!(
        cover.usage()[1].work - cover.usage()[0].work,
        measured.usage().work
    );
    boundary(measured.usage().work, |budget| {
        energy.pairs_cover(cycles(), &pi, budget).is_ok()
    })?;
    let parameters = parameters("1/4", "3/2")?;
    let gain = number("1")?;
    let mut measured = ledger()?;
    cover.witness(&parameters, gain.clone(), &mut measured)?;
    boundary(measured.usage().work, |budget| {
        cover.witness(&parameters, gain.clone(), budget).is_ok()
    })?;
    let mut measured = ledger()?;
    cover.calibration_at_energy(&parameters.energy, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        cover
            .calibration_at_energy(&parameters.energy, budget)
            .is_ok()
    })?;
    let mut measured = ledger()?;
    cover.reading_at_calibration(&gain, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        cover.reading_at_calibration(&gain, budget).is_ok()
    })?;
    assert!(energy.pairs_cover(cycles(), &pi, &mut ledger()?).is_ok());
    Ok(())
}

/// Unsupported physical scales, nonpositive calibrations/readings and incomplete cycles refuse.
/// ´test:integration:lc-relation-energy-only-calibration-refusals´
#[test]
fn lc_relation_energy_only_calibration_refusals() -> TestResult {
    let timing = timing(false, true, None)?;
    let unit = interval(["1", "1"])?;
    for bad in [
        interval(["0", "1"])?,
        interval(["-1", "1"])?,
        LcInterval::new(Some(number("1")?.into()), None, &mut ledger()?)?,
    ] {
        assert!(
            LcEnergyOnlySamples::new(timing.clone(), vec![bad.clone()], &unit, &mut ledger()?)
                .is_err()
        );
        assert!(
            LcEnergyOnlySamples::new(timing.clone(), samples(true)?, &bad, &mut ledger()?).is_err()
        );
    }
    assert!(LcEnergyOnlySamples::new(timing.clone(), Vec::new(), &unit, &mut ledger()?).is_err());
    let gain = interval(["9/10", "11/10"])?;
    for quantity in [
        Q::VoltageAmplitude,
        Q::CurrentAmplitude,
        Q::Inductance,
        Q::Capacitance,
    ] {
        let mut inputs = timing.relation().inputs().to_vec();
        inputs.push(input(quantity, ["1", "1"])?);
        let relation =
            LcRelation::new(timing.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
        let altered =
            LcSampleTimes::new(relation, timing.nominal_times().to_vec(), &mut ledger()?)?;
        let energy = LcEnergyOnlySamples::new(altered, samples(true)?, &gain, &mut ledger()?)?;
        assert!(
            energy
                .pairs_cover(cycles(), &pi()?, &mut ledger()?)
                .is_err()
        );
    }
    let energy = LcEnergyOnlySamples::new(timing, samples(true)?, &gain, &mut ledger()?)?;
    assert!(
        energy
            .pairs_cover(vec![number("0")?, number("1")?], &pi()?, &mut ledger()?)
            .is_err()
    );
    let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for voltage in ["0", "-1"] {
        assert!(
            cover
                .witness(&parameters("1/4", voltage)?, number("1")?, &mut ledger()?)
                .is_err()
        );
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Energy calibration controls retain both measurement layers and both physical amplitude channels.

use super::super::{
    TestResult, assert_bounds, cycles, interval, ledger, number, parameters, pi, question,
};
use nrz_bindings::{
    Budget, LcAmplitudeSamples, LcCertainty, LcConsistency, LcDeterminacy, LcEnergySamples,
    LcInterval, LcQuantity as Q, Limits, LimitsOverride,
};
use nrz_cas::Rational;

/// Both energy layers, each layer alone, and both measured amplitude channels have exact controls.
/// ´test:integration:lc-relation-energy-both-layers´
#[test]
fn lc_relation_energy_both_layers() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        for (per_sample, shared, expected) in [
            (true, true, ["9/40", "33/100"]),
            (true, false, ["1/4", "3/10"]),
            (false, true, ["9/40", "11/40"]),
        ] {
            let amplitude = question(channel, false, false, None)?;
            let energy = measured(amplitude.clone(), per_sample, shared)?;
            assert_eq!(energy.samples().len(), 2);
            assert!(energy.amplitude().shares_allocation(&amplitude));
            let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
            assert_bounds(&cover.bounds(Q::Energy, &mut ledger()?)?, expected)?;
            assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
            assert_eq!(cover.cycles(), cycles());
            assert_eq!(cover.certainty(), LcCertainty::Certified);
            assert_eq!(cover.consistency(), LcConsistency::Feasible);
            assert_eq!(
                cover.determinacy(Q::Energy)?,
                LcDeterminacy::DeterminedFinite
            );
            assert!(cover.energy().shares_allocation(&energy));
            assert!(cover.relation().shares_allocation(amplitude.relation()));
            let witness = cover.feasibility_witness();
            assert!(witness.energy().shares_allocation(&energy));
            assert!(witness.original().amplitude().shares_allocation(&amplitude));
            assert!(
                witness
                    .original()
                    .original()
                    .relation()
                    .shares_allocation(amplitude.relation())
            );
            energy.check_witness(
                witness.original().original().values().clone(),
                witness.original().gain().clone(),
                witness.calibration().clone(),
                &mut ledger()?,
            )?;
        }
    }
    Ok(())
}

/// Shared energy calibration has a conditional fibre; total boxes alone admit a false joint tuple.
/// ´test:integration:lc-relation-energy-calibration-shared-control´
#[test]
fn lc_relation_energy_calibration_shared_control() -> TestResult {
    let amplitude = question(Q::VoltageAmplitude, false, false, None)?;
    let energy = measured(amplitude.clone(), true, true)?;
    let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        cover.calibration_at_energy(&number("9/40")?, &mut ledger()?)?,
        [number("9/10")?, number("9/10")?]
    );
    assert_eq!(
        cover.calibration_at_energy(&number("33/100")?, &mut ledger()?)?,
        [number("11/10")?, number("11/10")?]
    );
    let witness = cover.witness(
        &parameters("1", "1", "9/40")?,
        number("9/10")?,
        &mut ledger()?,
    )?;
    amplitude.check_witness(
        witness.original().original().values().clone(),
        number("1")?,
        &mut ledger()?,
    )?;
    assert!(
        energy
            .check_witness(
                witness.original().original().values().clone(),
                number("1")?,
                number("11/10")?,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(
        cover
            .witness(
                &parameters("1", "1", "33/100")?,
                number("9/10")?,
                &mut ledger()?
            )
            .is_err()
    );
    // Independent total boxes [1,2] and [2,4] admit E=2 by using gains 2 and 1.
    let marginal = question(Q::VoltageAmplitude, false, false, Some(["2", "2"]))?;
    let point = marginal.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        point
            .feasibility_witness()
            .original()
            .values()
            .energy
            .as_rational(),
        Some(&number("2")?)
    );
    let physical = number("2")?;
    assert_eq!(physical.div(&number("1")?, &mut ledger()?)?, number("2")?);
    assert_eq!(physical.div(&number("2")?, &mut ledger()?)?, number("1")?);
    assert!(
        LcEnergySamples::new(
            amplitude,
            vec![interval(["1", "1"])?, interval(["2", "2"])?],
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Physical extrema and original witnesses agree with independent arbitrary-precision LC equations.
/// ´test:integration:lc-relation-energy-calibration-scale-oracles´
#[test]
fn lc_relation_energy_calibration_scale_oracles() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        for amplitude_layers in [false, true] {
            let amplitude = question(channel, amplitude_layers, amplitude_layers, None)?;
            let energy = measured(amplitude, true, true)?;
            let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
            let amplitude_points = if amplitude_layers {
                [("19/20", "9/10"), ("11/10", "11/10")]
            } else {
                [("1", "1"), ("1", "1")]
            };
            let mut extrema: [Option<[Rational; 2]>; 5] = [None, None, None, None, None];
            for (reading, gain) in amplitude_points {
                for (physical_energy, calibration) in [("9/40", "9/10"), ("33/100", "11/10")] {
                    let coordinates = oracle(channel, reading, gain, physical_energy)?;
                    for (range, value) in extrema.iter_mut().zip(coordinates) {
                        extend(range, value)?;
                    }
                    let witness = cover.witness(
                        &parameters(reading, gain, physical_energy)?,
                        number(calibration)?,
                        &mut ledger()?,
                    )?;
                    energy.check_witness(
                        witness.original().original().values().clone(),
                        number(gain)?,
                        number(calibration)?,
                        &mut ledger()?,
                    )?;
                }
            }
            for (quantity, expected) in [
                Q::Inductance,
                Q::Capacitance,
                Q::VoltageAmplitude,
                Q::CurrentAmplitude,
                Q::AmplitudeRatio,
            ]
            .into_iter()
            .zip(extrema)
            {
                let expected = expected.ok_or("missing independent extrema")?;
                let actual = cover.bounds(quantity, &mut ledger()?)?;
                assert_eq!(
                    actual
                        .lower()
                        .and_then(nrz_bindings::ExactConstant::as_rational),
                    Some(&expected[0])
                );
                assert_eq!(
                    actual
                        .upper()
                        .and_then(nrz_bindings::ExactConstant::as_rational),
                    Some(&expected[1])
                );
            }
            let witness =
                cover.witness(&parameters("1", "1", "1/4")?, number("1")?, &mut ledger()?)?;
            let mut crossed = witness.original().original().values().clone();
            crossed.energy = number("3/10")?.into();
            assert!(
                energy
                    .check_witness(crossed, number("1")?, number("1")?, &mut ledger()?)
                    .is_err()
            );
        }
    }
    Ok(())
}

/// Original total energy restrictions intersect the product image without discarding calibration.
/// ´test:integration:lc-relation-energy-calibration-original-intersection´
#[test]
fn lc_relation_energy_calibration_original_intersection() -> TestResult {
    let amplitude = question(Q::VoltageAmplitude, true, true, Some(["1/4", "3/10"]))?;
    let energy = measured(amplitude, true, true)?;
    assert_eq!(energy.energy(), &[number("1/4")?, number("3/10")?]);
    let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        cover.calibration_at_energy(&number("1/4")?, &mut ledger()?)?,
        [number("9/10")?, number("1")?]
    );
    assert!(
        cover
            .calibration_at_energy(&number("9/40")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(
                &parameters("1", "1", "9/40")?,
                number("9/10")?,
                &mut ledger()?
            )
            .is_err()
    );
    for (value, calibration) in [("1/4", "9/10"), ("11/40", "1"), ("3/10", "11/10")] {
        let witness = cover.witness(
            &parameters("1", "1", value)?,
            number(calibration)?,
            &mut ledger()?,
        )?;
        energy.check_witness(
            witness.original().original().values().clone(),
            number("1")?,
            number(calibration)?,
            &mut ledger()?,
        )?;
    }
    assert!(
        measured(
            question(Q::VoltageAmplitude, false, false, Some(["1/2", "1"]))?,
            true,
            true
        )
        .is_err()
    );
    Ok(())
}

/// Positivity, finite support, complete cycles and cumulative Work retain explicit refusals.
/// ´test:integration:lc-relation-energy-calibration-refusals-work´
#[test]
fn lc_relation_energy_calibration_refusals_work() -> TestResult {
    let amplitude = question(Q::VoltageAmplitude, true, true, None)?;
    for bad in [
        interval(["0", "1"])?,
        interval(["-1", "1"])?,
        LcInterval::new(Some(number("1")?.into()), None, &mut ledger()?)?,
    ] {
        assert!(
            LcEnergySamples::new(
                amplitude.clone(),
                vec![bad.clone()],
                &interval(["1", "1"])?,
                &mut ledger()?
            )
            .is_err()
        );
        assert!(
            LcEnergySamples::new(amplitude.clone(), samples(true)?, &bad, &mut ledger()?).is_err()
        );
    }
    assert!(
        LcEnergySamples::new(
            amplitude.clone(),
            Vec::new(),
            &interval(["1", "1"])?,
            &mut ledger()?
        )
        .is_err()
    );
    let readings = samples(true)?;
    let calibration = interval(["9/10", "11/10"])?;
    work_boundary(|budget| {
        Ok(LcEnergySamples::new(amplitude.clone(), readings.clone(), &calibration, budget).is_ok())
    })?;
    let energy = measured(amplitude, true, true)?;
    let pi = pi()?;
    assert!(
        energy
            .pairs_cover(vec![number("1")?, number("1")?], &pi, &mut ledger()?)
            .is_err()
    );
    let mut measured = ledger()?;
    let cover = energy.pairs_cover(cycles(), &pi, &mut measured)?;
    assert_eq!(
        cover.usage()[1].work - cover.usage()[0].work,
        measured.usage().work
    );
    assert!(cover.retained_payload_bytes() > energy.retained_payload_bytes());
    work_boundary(|budget| Ok(energy.pairs_cover(cycles(), &pi, budget).is_ok()))?;
    let point = parameters("1", "1", "1/4")?;
    let gain = number("1")?;
    work_boundary(|budget| Ok(cover.witness(&point, gain.clone(), budget).is_ok()))?;
    work_boundary(|budget| Ok(cover.calibration_at_energy(&point.energy, budget).is_ok()))?;
    work_boundary(|budget| Ok(cover.bounds(Q::Capacitance, budget).is_ok()))?;
    Ok(())
}

/// One increment of pre-spent Work separates success from refusal without changing any limit.
fn work_boundary(mut operation: impl FnMut(&mut Budget) -> TestResult<bool>) -> TestResult {
    let mut measured = ledger()?;
    assert!(operation(&mut measured)?);
    let work = measured.usage().work;
    assert!(work > 0);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?)?;
        budget.work(17, nrz_cas::Location::new("energy calibration prefix"))?;
        assert_eq!(operation(&mut budget)?, succeeds);
    }
    Ok(())
}

/// Independent equations derive both elements, both amplitudes and their ratio at fixed rate two.
fn oracle(channel: Q, reading: &str, gain: &str, energy: &str) -> TestResult<[Rational; 5]> {
    let mut budget = ledger()?;
    let amplitude = number(reading)?.mul(&number(gain)?, &mut budget)?;
    let direct = number("2")?
        .mul(&number(energy)?, &mut budget)?
        .div(&amplitude.mul(&amplitude, &mut budget)?, &mut budget)?;
    let inverse = number("1")?.div(&number("4")?.mul(&direct, &mut budget)?, &mut budget)?;
    let other = number("2")?
        .mul(&direct, &mut budget)?
        .mul(&amplitude, &mut budget)?;
    let (inductance, capacitance, voltage, current) = if channel == Q::VoltageAmplitude {
        (inverse, direct, amplitude, other)
    } else {
        (direct, inverse, other, amplitude)
    };
    let ratio = voltage.div(&current, &mut budget)?;
    Ok([inductance, capacitance, voltage, current, ratio])
}

/// Exact extrema use arbitrary-precision rational comparisons, with no native overflow oracle.
fn extend(range: &mut Option<[Rational; 2]>, value: Rational) -> TestResult {
    if let Some(range) = range {
        if value.compare(&range[0], &mut ledger()?)? == std::cmp::Ordering::Less {
            range[0].clone_from(&value);
        }
        if value.compare(&range[1], &mut ledger()?)? == std::cmp::Ordering::Greater {
            range[1].clone_from(&value);
        }
    } else {
        *range = Some([value.clone(), value]);
    }
    Ok(())
}

/// Unequal independent intervals refer to one conserved energy reading after calibration.
fn samples(per_sample: bool) -> TestResult<Vec<LcInterval>> {
    let ranges = if per_sample {
        [["1/5", "3/10"], ["1/4", "2/5"]]
    } else {
        [["1/4", "1/4"], ["1/4", "1/4"]]
    };
    ranges.into_iter().map(interval).collect()
}

/// The original amplitude owner is retained without replacing any supplied physical input.
fn measured(
    amplitude: LcAmplitudeSamples,
    per_sample: bool,
    shared: bool,
) -> TestResult<LcEnergySamples> {
    let gain = if shared {
        ["9/10", "11/10"]
    } else {
        ["1", "1"]
    };
    Ok(LcEnergySamples::new(
        amplitude,
        samples(per_sample)?,
        &interval(gain)?,
        &mut ledger()?,
    )?)
}

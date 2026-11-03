// SPDX-License-Identifier: AGPL-3.0-only
//! Original amplitude restrictions clip the physical image while retaining the conditional gain.

use super::{
    TestResult, assert_bounds, cycles, input, interval, ledger, number, parameters, pi, question,
};
use nrz_bindings::{
    Budget, LcAmplitudeCover, LcAmplitudeSamples, LcEnergyCover, LcEnergySamples, LcQuantity as Q,
    LcRelation, LcSampleTimes, Limits, LimitsOverride,
};

/// Two original physical restrictions jointly clip the gain/sample image to [1,11/10].
fn narrow(channel: Q, energy: bool) -> TestResult<LcAmplitudeSamples> {
    let original = question(channel, true, true, energy.then_some(["1/4", "1/4"]))?;
    let mut inputs = original.relation().inputs().to_vec();
    inputs.push(input(channel, ["1", "6/5"])?);
    inputs.push(input(channel, ["9/10", "11/10"])?);
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
    Ok(LcAmplitudeSamples::new(
        timing,
        channel,
        original.samples().to_vec(),
        &interval(["9/10", "11/10"])?,
        &mut ledger()?,
    )?)
}

/// Both physical channels use exact clipped extrema and retain nontrivial conditional gain fibres.
/// ´test:integration:lc-relation-narrower-original-amplitude´
#[test]
fn lc_relation_narrower_original_amplitude() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let amplitude = narrow(channel, true)?;
        assert_eq!(
            amplitude.physical_amplitude(),
            &[number("1")?, number("11/10")?]
        );
        assert_eq!(amplitude.measured(), &[number("19/20")?, number("11/10")?]);
        assert_eq!(amplitude.gain(), &[number("9/10")?, number("11/10")?]);
        assert_eq!(amplitude.samples().len(), 2);
        let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_bounds(&cover.bounds(channel, &mut ledger()?)?, ["1", "11/10"])?;
        assert_eq!(
            cover.gain_at_amplitude(&number("1")?, &mut ledger()?)?,
            [number("10/11")?, number("20/19")?]
        );
        assert_eq!(
            cover.gain_at_amplitude(&number("11/10")?, &mut ledger()?)?,
            [number("1")?, number("11/10")?]
        );
        physical_bounds(&cover, channel)?;
        witness_controls(&cover, &amplitude)?;
    }
    Ok(())
}

/// Literal physical extrema are independent of the production monomial elimination.
fn physical_bounds(cover: &LcAmplitudeCover, channel: Q) -> TestResult {
    let (direct, inverse, other, ratio) = if channel == Q::VoltageAmplitude {
        (
            Q::Capacitance,
            Q::Inductance,
            Q::CurrentAmplitude,
            ["1", "121/100"],
        )
    } else {
        (
            Q::Inductance,
            Q::Capacitance,
            Q::VoltageAmplitude,
            ["100/121", "1"],
        )
    };
    for (quantity, expected) in [
        (direct, ["50/121", "1/2"]),
        (inverse, ["1/2", "121/200"]),
        (other, ["10/11", "1"]),
        (Q::AmplitudeRatio, ratio),
    ] {
        assert_bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
    }
    Ok(())
}

/// Reconstructed tuples preserve original identity and reject crossed conditional gains.
fn witness_controls(cover: &LcAmplitudeCover, amplitude: &LcAmplitudeSamples) -> TestResult {
    for (measured, gain) in [
        ("11/10", "10/11"),
        ("19/20", "20/19"),
        ("11/10", "1"),
        ("1", "11/10"),
    ] {
        let witness = cover.witness(&parameters(measured, gain, "1/4")?, &mut ledger()?)?;
        assert!(witness.amplitude().shares_allocation(amplitude));
        assert!(
            witness
                .original()
                .relation()
                .shares_allocation(amplitude.relation())
        );
        amplitude.check_witness(
            witness.original().values().clone(),
            witness.gain().clone(),
            &mut ledger()?,
        )?;
    }
    assert!(
        cover
            .gain_at_amplitude(&number("171/200")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(&parameters("19/20", "9/10", "1/4")?, &mut ledger()?)
            .is_err()
    );
    let point = cover.witness(&parameters("1", "1", "1/4")?, &mut ledger()?)?;
    for gain in ["9/10", "11/10"] {
        assert!(
            amplitude
                .check_witness(
                    point.original().values().clone(),
                    number(gain)?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    Ok(())
}

/// Clipped amplitude and shared energy calibration retain both original conditional fibres.
/// ´test:integration:lc-relation-narrower-amplitude-energy-calibration´
#[test]
fn lc_relation_narrower_amplitude_energy_calibration() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let amplitude = narrow(channel, false)?;
        let energy = LcEnergySamples::new(
            amplitude.clone(),
            vec![interval(["1/5", "3/10"])?, interval(["1/4", "2/5"])?],
            &interval(["9/10", "11/10"])?,
            &mut ledger()?,
        )?;
        let cover = energy.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_bounds(&cover.bounds(channel, &mut ledger()?)?, ["1", "11/10"])?;
        assert_bounds(
            &cover.bounds(Q::Energy, &mut ledger()?)?,
            ["9/40", "33/100"],
        )?;
        calibrated_bounds(&cover, channel)?;
        for (measured, gain, physical_energy, calibration) in [
            ("11/10", "10/11", "9/40", "9/10"),
            ("1", "11/10", "33/100", "11/10"),
        ] {
            let witness = cover.witness(
                &parameters(measured, gain, physical_energy)?,
                number(calibration)?,
                &mut ledger()?,
            )?;
            assert!(witness.energy().shares_allocation(&energy));
            assert!(witness.original().amplitude().shares_allocation(&amplitude));
            energy.check_witness(
                witness.original().original().values().clone(),
                witness.original().gain().clone(),
                witness.calibration().clone(),
                &mut ledger()?,
            )?;
        }
        assert!(
            cover
                .witness(
                    &parameters("11/10", "10/11", "33/100")?,
                    number("9/10")?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    Ok(())
}

/// Independent physical extrema use the clipped amplitude with both original energy layers.
fn calibrated_bounds(cover: &LcEnergyCover, channel: Q) -> TestResult {
    let (direct, inverse, other) = if channel == Q::VoltageAmplitude {
        (Q::Capacitance, Q::Inductance, Q::CurrentAmplitude)
    } else {
        (Q::Inductance, Q::Capacitance, Q::VoltageAmplitude)
    };
    for (quantity, expected) in [
        (direct, ["45/121", "33/50"]),
        (inverse, ["25/66", "121/180"]),
        (other, ["9/11", "33/25"]),
    ] {
        assert_bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
    }
    Ok(())
}

/// Joint admission, conditional fibres and reconstruction keep exact cumulative resource boundaries.
/// ´test:integration:lc-relation-narrower-amplitude-refusals-work´
#[test]
fn lc_relation_narrower_amplitude_refusals_work() -> TestResult {
    let amplitude = narrow(Q::VoltageAmplitude, true)?;
    let gain = interval(["9/10", "11/10"])?;
    let mut measured = ledger()?;
    LcAmplitudeSamples::new(
        amplitude.timing().clone(),
        amplitude.channel(),
        amplitude.samples().to_vec(),
        &gain,
        &mut measured,
    )?;
    boundary(measured.usage().work, |budget| {
        LcAmplitudeSamples::new(
            amplitude.timing().clone(),
            amplitude.channel(),
            amplitude.samples().to_vec(),
            &gain,
            budget,
        )
        .is_ok()
    })?;
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = amplitude.pairs_cover(cycles(), &pi, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        amplitude.pairs_cover(cycles(), &pi, budget).is_ok()
    })?;
    let physical = number("1")?;
    let mut measured = ledger()?;
    cover.gain_at_amplitude(&physical, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        cover.gain_at_amplitude(&physical, budget).is_ok()
    })?;
    let parameters = parameters("11/10", "10/11", "1/4")?;
    let mut measured = ledger()?;
    cover.witness(&parameters, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        cover.witness(&parameters, budget).is_ok()
    })?;
    disjoint_originals(&amplitude, &gain)?;
    Ok(())
}

/// Disjoint and zero original amplitude supports cannot create a joint positive measurement.
fn disjoint_originals(
    amplitude: &LcAmplitudeSamples,
    gain: &nrz_bindings::LcInterval,
) -> TestResult {
    for range in [["2", "3"], ["0", "0"]] {
        let mut inputs = amplitude.relation().inputs().to_vec();
        inputs.push(input(amplitude.channel(), range)?);
        let relation = LcRelation::new(
            amplitude.relation().epoch().into(),
            2,
            inputs,
            &mut ledger()?,
        )?;
        let timing = LcSampleTimes::new(
            relation,
            amplitude.timing().nominal_times().to_vec(),
            &mut ledger()?,
        )?;
        assert!(
            LcAmplitudeSamples::new(
                timing,
                amplitude.channel(),
                amplitude.samples().to_vec(),
                gain,
                &mut ledger()?
            )
            .is_err()
        );
    }
    Ok(())
}

/// No failed attempt refunds the caller's spent prefix or raises the existing default limit.
fn boundary(work: u64, operation: impl Fn(&mut Budget) -> bool) -> TestResult {
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, nrz_cas::Location::new("narrow amplitude prefix"))?;
        assert_eq!(operation(&mut budget), success);
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

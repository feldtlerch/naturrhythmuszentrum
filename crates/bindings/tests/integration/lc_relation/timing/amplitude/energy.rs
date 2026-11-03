// SPDX-License-Identifier: AGPL-3.0-only
//! Repeated positive energy intervals remain one physical energy coordinate beside amplitude gain.

mod calibrated;

use super::{
    TestResult, assert_bounds, cycles, input, interval, ledger, number, parameters, pi, question,
};
use nrz_bindings::{
    LcAmplitudeSamples, LcDeterminacy, LcInterval, LcQuantity as Q, LcRelation, LcSampleTimes,
};

/// Independent repeated energy observations intersect while every original declaration is retained.
/// ´test:integration:lc-relation-energy-sample-intervals´
#[test]
fn lc_relation_energy_sample_intervals() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let amplitude = repeated(channel, false)?;
        assert_eq!(amplitude.energy(), Some(&[number("1/4")?, number("3/10")?]));
        let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        let (direct, inverse, other, ratio) = if channel == Q::VoltageAmplitude {
            (
                Q::Capacitance,
                Q::Inductance,
                Q::CurrentAmplitude,
                ["5/6", "1"],
            )
        } else {
            (
                Q::Inductance,
                Q::Capacitance,
                Q::VoltageAmplitude,
                ["1", "6/5"],
            )
        };
        for (quantity, expected) in [
            (Q::Energy, ["1/4", "3/10"]),
            (direct, ["1/2", "3/5"]),
            (inverse, ["5/12", "1/2"]),
            (other, ["1", "6/5"]),
            (Q::AmplitudeRatio, ratio),
        ] {
            assert_bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
            assert_eq!(
                cover.determinacy(quantity)?,
                LcDeterminacy::DeterminedFinite
            );
        }
        for energy in ["1/4", "11/40", "3/10"] {
            let witness = cover.witness(&parameters("1", "1", energy)?, &mut ledger()?)?;
            amplitude.check_witness(
                witness.original().values().clone(),
                witness.gain().clone(),
                &mut ledger()?,
            )?;
        }
        assert!(
            cover
                .witness(&parameters("1", "1", "1/5")?, &mut ledger()?)
                .is_err()
        );
        assert!(
            cover
                .witness(&parameters("1", "1", "2/5")?, &mut ledger()?)
                .is_err()
        );
    }
    Ok(())
}

/// Energy and both amplitude layers share their original equations rather than independent marginals.
/// ´test:integration:lc-relation-energy-amplitude-joint´
#[test]
fn lc_relation_energy_amplitude_joint() -> TestResult {
    let amplitude = repeated(Q::VoltageAmplitude, true)?;
    let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for (quantity, expected) in [
        (Q::VoltageAmplitude, ["171/200", "121/100"]),
        (Q::Capacitance, ["5000/14641", "24000/29241"]),
        (Q::Inductance, ["29241/96000", "14641/20000"]),
        (Q::CurrentAmplitude, ["100/121", "80/57"]),
        (Q::AmplitudeRatio, ["29241/48000", "14641/10000"]),
    ] {
        assert_bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
    }
    for (measured, gain, energy) in [("19/20", "9/10", "3/10"), ("11/10", "11/10", "1/4")] {
        let witness = cover.witness(&parameters(measured, gain, energy)?, &mut ledger()?)?;
        amplitude.check_witness(
            witness.original().values().clone(),
            witness.gain().clone(),
            &mut ledger()?,
        )?;
    }
    let witness = cover.witness(&parameters("1", "1", "1/4")?, &mut ledger()?)?;
    let mut crossed = witness.original().values().clone();
    crossed.energy = number("3/10")?.into();
    assert!(
        amplitude
            .check_witness(crossed, number("1")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Tightening one energy sample preserves input inclusion and every physical marginal bound.
/// ´test:integration:lc-relation-energy-refinement´
#[test]
fn lc_relation_energy_refinement() -> TestResult {
    let previous = repeated(Q::VoltageAmplitude, true)?;
    let mut inputs = previous.relation().inputs().to_vec();
    inputs.push(input(Q::Energy, ["13/50", "7/25"])?);
    let relation = LcRelation::new(
        previous.relation().epoch().into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let inclusion = previous
        .relation()
        .refinement_inputs(&relation, &mut ledger()?)?;
    assert!(inclusion.previous().shares_allocation(previous.relation()));
    let timing = LcSampleTimes::new(
        relation,
        previous.timing().nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    let refined = LcAmplitudeSamples::new(
        timing,
        previous.channel(),
        previous.samples().to_vec(),
        &interval(["9/10", "11/10"])?,
        &mut ledger()?,
    )?;
    let old = previous.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let new = refined.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(old.rate(), new.rate());
    for quantity in [
        Q::Energy,
        Q::Inductance,
        Q::Capacitance,
        Q::AmplitudeRatio,
        Q::VoltageAmplitude,
        Q::CurrentAmplitude,
    ] {
        let outer = old.bounds(quantity, &mut ledger()?)?;
        let inner = new.bounds(quantity, &mut ledger()?)?;
        assert_included(&outer, &inner)?;
    }
    Ok(())
}

/// Positive interval admission and cumulative Work retain their explicit resource and branch limits.
/// ´test:integration:lc-relation-energy-refusals-work´
#[test]
fn lc_relation_energy_refusals_work() -> TestResult {
    use nrz_bindings::{Budget, Limits, LimitsOverride};
    for energy in [["0", "0"], ["0", "1/4"], ["-1", "1/4"]] {
        assert!(question(Q::VoltageAmplitude, true, true, Some(energy)).is_err());
    }
    let amplitude = repeated(Q::VoltageAmplitude, true)?;
    let mut inputs = amplitude.relation().inputs().to_vec();
    inputs.push(input(Q::Energy, ["1/2", "3/5"])?);
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
            &interval(["9/10", "11/10"])?,
            &mut ledger()?
        )
        .is_err()
    );
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = amplitude.pairs_cover(cycles(), &pi, &mut measured)?;
    let work = measured.usage().work;
    assert_eq!(cover.usage()[1].work - cover.usage()[0].work, work);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, nrz_cas::Location::new("energy samples prefix"))?;
        assert_eq!(
            amplitude.pairs_cover(cycles(), &pi, &mut budget).is_ok(),
            succeeds
        );
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

/// Two independent energy samples retain the intersection `[1/4,3/10]` of their original boxes.
fn repeated(channel: Q, uncertain_amplitude: bool) -> TestResult<LcAmplitudeSamples> {
    let previous = question(
        channel,
        uncertain_amplitude,
        uncertain_amplitude,
        Some(["1/5", "3/10"]),
    )?;
    let mut inputs = previous.relation().inputs().to_vec();
    inputs.push(input(Q::Energy, ["1/4", "2/5"])?);
    let relation = LcRelation::new(
        previous.relation().epoch().into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let timing = LcSampleTimes::new(
        relation,
        previous.timing().nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    let gain = if uncertain_amplitude {
        ["9/10", "11/10"]
    } else {
        ["1", "1"]
    };
    Ok(LcAmplitudeSamples::new(
        timing,
        channel,
        previous.samples().to_vec(),
        &interval(gain)?,
        &mut ledger()?,
    )?)
}

/// Public exact rational endpoints prove both sides of the projected interval inclusion.
fn assert_included(outer: &LcInterval, inner: &LcInterval) -> TestResult {
    for (lower, upper) in [
        (outer.lower(), inner.lower()),
        (inner.upper(), outer.upper()),
    ] {
        let lower = lower
            .and_then(nrz_bindings::ExactConstant::as_rational)
            .ok_or("finite rational inclusion lower endpoint")?;
        let upper = upper
            .and_then(nrz_bindings::ExactConstant::as_rational)
            .ok_or("finite rational inclusion upper endpoint")?;
        assert_ne!(
            lower.compare(upper, &mut ledger()?)?,
            std::cmp::Ordering::Greater
        );
    }
    Ok(())
}

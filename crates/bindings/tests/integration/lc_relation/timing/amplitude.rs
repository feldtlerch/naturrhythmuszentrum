// SPDX-License-Identifier: AGPL-3.0-only
//! Exact amplitude controls distinguish per-measurement support from one common gain.

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    LcAmplitudeParameters, LcAmplitudeSamples, LcDeterminacy, LcFreeDirection, LcInterval,
    LcQuantity as Q, LcRelation, LcSampleTimes,
};

/// Each amplitude layer has a distinct projection, and their composition retains both.
/// ´test:integration:lc-relation-amplitude-both-layers´
#[test]
fn lc_relation_amplitude_both_layers() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        for (per_sample, shared, expected) in [
            (true, true, ["171/200", "121/100"]),
            (true, false, ["19/20", "11/10"]),
            (false, true, ["9/10", "11/10"]),
        ] {
            let amplitude = question(channel, per_sample, shared, Some(["1/4", "1/4"]))?;
            let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
            assert_bounds(&cover.bounds(channel, &mut ledger()?)?, expected)?;
            assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
            assert!(cover.amplitude().shares_allocation(&amplitude));
            let witness = cover.feasibility_witness();
            amplitude.check_witness(
                witness.original().values().clone(),
                witness.gain().clone(),
                &mut ledger()?,
            )?;
        }
    }
    Ok(())
}

/// Positive monomial extrema agree with independent rational LC eliminations on both channels.
/// ´test:integration:lc-relation-amplitude-scale-oracles´
#[test]
fn lc_relation_amplitude_scale_oracles() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let amplitude = question(channel, true, true, Some(["1/4", "1/4"]))?;
        let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        let (direct, inverse, other, ratio) = if channel == Q::VoltageAmplitude {
            (
                Q::Capacitance,
                Q::Inductance,
                Q::CurrentAmplitude,
                ["29241/40000", "14641/10000"],
            )
        } else {
            (
                Q::Inductance,
                Q::Capacitance,
                Q::VoltageAmplitude,
                ["10000/14641", "40000/29241"],
            )
        };
        for (quantity, expected) in [
            (direct, ["5000/14641", "20000/29241"]),
            (inverse, ["29241/80000", "14641/20000"]),
            (other, ["100/121", "200/171"]),
            (Q::AmplitudeRatio, ratio),
            (Q::Energy, ["1/4", "1/4"]),
        ] {
            assert_bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
            assert_eq!(
                cover.determinacy(quantity)?,
                LcDeterminacy::DeterminedFinite
            );
        }
        for (measured, gain) in [("19/20", "9/10"), ("11/10", "11/10")] {
            let witness = cover.witness(&parameters(measured, gain, "1/4")?, &mut ledger()?)?;
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
    }
    Ok(())
}

/// Gain and physical-amplitude marginals cannot be freely crossed; independent gains differ.
/// ´test:integration:lc-relation-amplitude-shared-control´
#[test]
fn lc_relation_amplitude_shared_control() -> TestResult {
    let amplitude = question(Q::VoltageAmplitude, true, true, Some(["1/4", "1/4"]))?;
    let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        cover.gain_at_amplitude(&number("171/200")?, &mut ledger()?)?,
        [number("9/10")?, number("9/10")?]
    );
    let witness = cover.witness(&parameters("19/20", "9/10", "1/4")?, &mut ledger()?)?;
    amplitude
        .relation()
        .check_witness(witness.original().values().clone(), &mut ledger()?)?;
    assert!(
        amplitude
            .check_witness(
                witness.original().values().clone(),
                number("11/10")?,
                &mut ledger()?
            )
            .is_err()
    );
    // Separate gains can reconcile disjoint measured singletons with the same physical amplitude.
    let physical = number("2")?;
    assert_eq!(physical.div(&number("2")?, &mut ledger()?)?, number("1")?);
    assert_eq!(physical.div(&number("1")?, &mut ledger()?)?, number("2")?);
    let disjoint = vec![interval(["1", "1"])?, interval(["2", "2"])?];
    assert!(
        LcAmplitudeSamples::new(
            amplitude.timing().clone(),
            Q::VoltageAmplitude,
            disjoint,
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Measured amplitude without energy leaves the element scale free instead of fabricating bounds.
/// ´test:integration:lc-relation-amplitude-free-scale´
#[test]
fn lc_relation_amplitude_free_scale() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let amplitude = question(channel, true, true, None)?;
        let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_eq!(cover.determinacy(channel)?, LcDeterminacy::DeterminedFinite);
        let other = if channel == Q::VoltageAmplitude {
            Q::CurrentAmplitude
        } else {
            Q::VoltageAmplitude
        };
        for quantity in [
            Q::Inductance,
            Q::Capacitance,
            Q::AmplitudeRatio,
            other,
            Q::Energy,
        ] {
            assert_eq!(cover.determinacy(quantity)?, LcDeterminacy::Underdetermined);
            assert_eq!(
                cover.free_directions(quantity)?,
                &[LcFreeDirection::ElementScale]
            );
            let bounds = cover.bounds(quantity, &mut ledger()?)?;
            assert_eq!(
                bounds
                    .lower()
                    .and_then(nrz_bindings::ExactConstant::as_rational),
                Some(&number("0")?)
            );
            assert!(bounds.upper().is_none());
        }
        for energy in ["1/4", "1", "100"] {
            let witness = cover.witness(&parameters("1", "1", energy)?, &mut ledger()?)?;
            amplitude.check_witness(
                witness.original().values().clone(),
                witness.gain().clone(),
                &mut ledger()?,
            )?;
        }
    }
    Ok(())
}

/// Original-image restrictions, omitted simultaneous scales and resource failures refuse explicitly.
/// ´test:integration:lc-relation-amplitude-refusals-work´
#[test]
fn lc_relation_amplitude_refusals_work() -> TestResult {
    use nrz_bindings::{Budget, Limits, LimitsOverride};
    let amplitude = question(Q::VoltageAmplitude, true, true, Some(["1/4", "1/4"]))?;
    for extra in [
        input(Q::VoltageAmplitude, ["1", "1"])?,
        input(Q::Capacitance, ["1", "1"])?,
    ] {
        let mut inputs = amplitude.relation().inputs().to_vec();
        inputs.push(extra);
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
                Q::VoltageAmplitude,
                samples(true)?,
                &interval(["9/10", "11/10"])?,
                &mut ledger()?
            )
            .is_err()
        );
    }
    assert!(
        LcAmplitudeSamples::new(
            amplitude.timing().clone(),
            Q::VoltageAmplitude,
            Vec::new(),
            &interval(["1", "1"])?,
            &mut ledger()?
        )
        .is_err()
    );
    assert!(
        LcAmplitudeSamples::new(
            amplitude.timing().clone(),
            Q::VoltageAmplitude,
            samples(true)?,
            &interval(["0", "1"])?,
            &mut ledger()?
        )
        .is_err()
    );
    let pi = pi()?;
    let mut measured = ledger()?;
    let cover = amplitude.pairs_cover(cycles(), &pi, &mut measured)?;
    let work = measured.usage().work;
    assert_eq!(cover.usage()[1].work - cover.usage()[0].work, work);
    assert!(cover.retained_payload_bytes() > 0);
    for (limit, succeeds) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(17, nrz_cas::Location::new("amplitude layers prefix"))?;
        assert_eq!(
            amplitude.pairs_cover(cycles(), &pi, &mut budget).is_ok(),
            succeeds
        );
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

/// Exact phase observations identify the rate independently of the retained scale coordinates.
fn question(
    channel: Q,
    per_sample: bool,
    shared: bool,
    energy: Option<[&str; 2]>,
) -> TestResult<LcAmplitudeSamples> {
    let mut inputs = vec![
        input(Q::Rate, ["1", "3"])?,
        input(Q::InitialPhase, ["0", "0"])?,
        input(channel, ["1/2", "3"])?,
    ];
    for (index, (time, phase)) in [("1", "2"), ("2", "4")].into_iter().enumerate() {
        inputs.push(input(Q::Time(index), [time, time])?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    if let Some(energy) = energy {
        inputs.push(input(Q::Energy, energy)?);
    }
    let relation = LcRelation::new(
        "amplitude measurement epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let timing = LcSampleTimes::new(relation, vec![number("1")?, number("2")?], &mut ledger()?)?;
    let gain = if shared {
        ["9/10", "11/10"]
    } else {
        ["1", "1"]
    };
    Ok(LcAmplitudeSamples::new(
        timing,
        channel,
        samples(per_sample)?,
        &interval(gain)?,
        &mut ledger()?,
    )?)
}

/// Unequal repeated sample supports describe one constant autonomous amplitude.
fn samples(per_sample: bool) -> TestResult<Vec<LcInterval>> {
    let ranges = if per_sample {
        [["9/10", "11/10"], ["19/20", "23/20"]]
    } else {
        [["1", "1"], ["1", "1"]]
    };
    ranges.into_iter().map(interval).collect()
}

/// Exact deterministic supports carry coherent scalar amplitude coordinates.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

/// Every chosen physical coordinate is used by the inverse reconstruction.
fn parameters(measured: &str, gain: &str, energy: &str) -> TestResult<LcAmplitudeParameters> {
    Ok(LcAmplitudeParameters {
        rate: number("2")?,
        initial_phase: number("0")?,
        gauge: number("0")?,
        measured: number(measured)?,
        gain: number(gain)?,
        energy: number(energy)?,
    })
}

/// Literal independent rational extrema are compared to exact coherent endpoints.
fn assert_bounds(actual: &LcInterval, expected: [&str; 2]) -> TestResult {
    assert_eq!(
        actual
            .lower()
            .and_then(nrz_bindings::ExactConstant::as_rational),
        Some(&number(expected[0])?)
    );
    assert_eq!(
        actual
            .upper()
            .and_then(nrz_bindings::ExactConstant::as_rational),
        Some(&number(expected[1])?)
    );
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Narrow original amplitude bounds compose with the same original clock and phase calibrations.

use super::{
    TestResult, assert_bounds, check_physical, cycles, input, interval, ledger, number, parameters,
    pi, source, work_boundary,
};
use nrz_bindings::{LcPhaseGauge, LcQuantity as Q, LcSharedMeasurementCover, LcSharedMeasurements};

/// The original shared clock, phase offset and amplitude gain remain one joint owner after clipping.
/// ´test:integration:lc-relation-narrower-amplitude-three-shared-errors´
#[test]
fn lc_relation_narrower_amplitude_three_shared_errors() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        for known in [false, true] {
            let phase = source(
                channel,
                known,
                true,
                vec![
                    input(channel, ["1", "6/5"])?,
                    input(channel, ["9/10", "11/10"])?,
                ],
            )?;
            let samples = vec![interval(["9/10", "11/10"])?, interval(["19/20", "23/20"])?];
            let question = LcSharedMeasurements::new(
                phase.clone(),
                channel,
                samples,
                &interval(["9/10", "11/10"])?,
                &mut ledger()?,
            )?;
            assert!(question.phase().shares_allocation(&phase));
            assert_eq!(
                question.physical_amplitude(),
                &[number("1")?, number("11/10")?]
            );
            let cover = question.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
            assert!(cover.measurements().shares_allocation(&question));
            assert_eq!(
                cover.phase_gauge(),
                if known {
                    LcPhaseGauge::Fixed
                } else {
                    LcPhaseGauge::AllIntegers
                }
            );
            assert_bounds(&cover.bounds(channel, &mut ledger()?)?, ["1", "11/10"])?;
            assert_eq!(
                cover.gain_at_amplitude(&number("1")?, &mut ledger()?)?,
                [number("10/11")?, number("20/19")?]
            );
            check_physical(&question, cover.feasibility_witness())?;
            witnesses(&cover, &question, known)?;
        }
    }
    Ok(())
}

/// Every selected systematic coordinate is rechecked on the same complete original question.
fn witnesses(
    cover: &LcSharedMeasurementCover,
    question: &LcSharedMeasurements,
    known: bool,
) -> TestResult {
    for (clock, phase) in [("-1/10", "-1/5"), ("0", "0"), ("1/10", "1/5")] {
        let gauge = if known { "0" } else { "2" };
        let parameters = parameters("0", clock, phase, "11/10", "10/11", gauge)?;
        let point = cover.witness(&parameters, &mut ledger()?)?;
        check_physical(question, &point)?;
        assert!(point.original().phase().shares_allocation(question.phase()));
        let mut crossed = parameters.clone();
        crossed.gain = number("9/10")?;
        assert!(cover.witness(&crossed, &mut ledger()?).is_err());
        crossed = parameters;
        crossed.phase_offset = number("3/10")?;
        assert!(cover.witness(&crossed, &mut ledger()?).is_err());
    }
    Ok(())
}

/// The combined owner charges clipping, joint recovery, conditional gain and original reconstruction.
/// ´test:integration:lc-relation-narrower-amplitude-three-error-work´
#[test]
fn lc_relation_narrower_amplitude_three_error_work() -> TestResult {
    let phase = source(
        Q::VoltageAmplitude,
        true,
        true,
        vec![input(Q::VoltageAmplitude, ["1", "11/10"])?],
    )?;
    let samples = vec![interval(["9/10", "11/10"])?, interval(["19/20", "23/20"])?];
    let gain = interval(["9/10", "11/10"])?;
    work_boundary(|budget| {
        LcSharedMeasurements::new(
            phase.clone(),
            Q::VoltageAmplitude,
            samples.clone(),
            &gain,
            budget,
        )
    })?;
    let question =
        LcSharedMeasurements::new(phase, Q::VoltageAmplitude, samples, &gain, &mut ledger()?)?;
    let pi = pi()?;
    work_boundary(|budget| question.pairs_cover(cycles(), &pi, budget))?;
    let cover = question.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let physical = number("1")?;
    work_boundary(|budget| cover.gain_at_amplitude(&physical, budget))?;
    let parameters = parameters("0", "1/10", "1/5", "11/10", "10/11", "0")?;
    work_boundary(|budget| cover.witness(&parameters, budget))?;
    Ok(())
}

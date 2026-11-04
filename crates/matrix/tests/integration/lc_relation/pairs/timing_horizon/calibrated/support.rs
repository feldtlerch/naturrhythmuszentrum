// SPDX-License-Identifier: AGPL-3.0-only
//! Dyadic original calibration domains give independently derived affine-pi future thresholds.

use super::{TestResult, input, ledger, number};
use nrz_bindings::{
    LcInterval, LcQuantity as Q, LcRelation, LcSharedClock, LcSharedClockPhase,
    LcSharedMeasurements,
};

/// All original coordinates persist while physical phase and both systematic offsets narrow.
pub(super) fn phase(tight: bool, channel: Option<Q>) -> TestResult<LcSharedClockPhase> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if let Some(channel) = channel {
        inputs.push(input(channel, ["1", "9/8"])?);
        inputs.push(input(Q::Energy, ["1/2", "1/2"])?);
    }
    let physical = if tight {
        [["15/8", "17/8"], ["31/8", "33/8"]]
    } else {
        [["7/4", "9/4"], ["15/4", "17/4"]]
    };
    for (index, (time, phase)) in [["1/2", "3/2"], ["3/2", "5/2"]]
        .into_iter()
        .zip(physical)
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    let relation = LcRelation::new("calibrated future epoch".into(), 2, inputs, &mut ledger()?)?;
    let offset = if tight {
        ["-1/8", "1/8"]
    } else {
        ["-1/4", "1/4"]
    };
    let clock = LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &interval(offset)?,
        &mut ledger()?,
    )?;
    let samples = if tight {
        [["7/4", "9/4"], ["15/4", "17/4"]]
    } else {
        [["3/2", "5/2"], ["7/2", "9/2"]]
    };
    Ok(LcSharedClockPhase::new(
        clock,
        samples
            .into_iter()
            .map(interval)
            .collect::<TestResult<Vec<_>>>()?,
        &interval(offset)?,
        &mut ledger()?,
    )?)
}

/// Both physical amplitude channels keep repeated readings and one shared calibration.
pub(super) fn amplitude(tight: bool, channel: Q) -> TestResult<LcSharedMeasurements> {
    let readings = if tight {
        ["7/8", "9/8"]
    } else {
        ["3/4", "5/4"]
    };
    let gain = if tight {
        ["3/4", "5/4"]
    } else {
        ["1/2", "3/2"]
    };
    Ok(LcSharedMeasurements::new(
        phase(tight, Some(channel))?,
        channel,
        vec![interval(readings)?, interval(readings)?],
        &interval(gain)?,
        &mut ledger()?,
    )?)
}

/// Signed exact finite declarations are independent of native numerical formats.
pub(super) fn interval(bounds: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(bounds[0])?.into()),
        Some(number(bounds[1])?.into()),
        &mut ledger()?,
    )?)
}

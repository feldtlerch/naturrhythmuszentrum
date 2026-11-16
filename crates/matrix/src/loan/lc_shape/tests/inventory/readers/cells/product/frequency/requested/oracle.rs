// SPDX-License-Identifier: AGPL-3.0-only
//! Rational squared and inverse-squared inequalities use independent literal full-turn bounds.

use super::super::{
    Rational, TestResult, check as enclosure,
    quantities::{local, mul},
};
use crate::{LcRequestedCell, LcRequestedState};
use nrz_scalar::Scalar;

pub(super) fn check<P: Scalar + 'static>(result: &LcRequestedCell<'_, '_, P, 5>) -> TestResult {
    let LcRequestedState::Ready { tau, rate, values } = result.state() else {
        return Err("ready requested path".into());
    };
    // Interpret the independently specified trace vocabulary, not the production selector.
    let label = super::super::observation::spec(result.demand()).2;
    let [omega, frequency, period] = ["omega", "frequency", "period"]
        .map(|quantity| label.split('-').any(|part| part == quantity));
    assert_eq!(result.demand().selection(), [omega, frequency, period]);
    assert_eq!(
        [
            values.omega.is_some(),
            values.frequency.is_some(),
            values.period.is_some()
        ],
        [omega, frequency, period]
    );
    assert_eq!(rate.is_some(), omega || frequency);
    let [left, right] = result
        .positions()
        .map(|position| result.inputs().readings()[position].values());
    let [left, right] = [left, right].map(|value| {
        Ok::<_, Box<dyn std::error::Error>>([
            Rational::binary(value[0].to_f64())?,
            Rational::binary(value[1].to_f64())?,
        ])
    });
    let [left, right] = [left?, right?];
    let low = mul(&left[0].sub(&left[1]), &right[0].sub(&right[1]))?;
    let high = mul(&left[0].add(&left[1]), &right[0].add(&right[1]))?;
    enclosure(tau, &low, &high, false)?;
    if let Some(rate) = rate {
        enclosure(rate, &low, &high, true)?;
    }
    if omega {
        assert_eq!(values.omega, rate);
    }
    let turn_low = Rational::fraction(6_283_185_307_179_586_476, 1_000_000_000_000_000_000)?;
    let turn_high = Rational::fraction(6_283_185_307_179_586_478, 1_000_000_000_000_000_000)?;
    let low = mul(&low, &mul(&turn_low, &turn_low)?)?;
    let high = mul(&high, &mul(&turn_high, &turn_high)?)?;
    if let Some(value) = values.frequency {
        enclosure(value, &low, &high, true)?;
        local(
            value,
            rate.ok_or("rate dependency")?[0],
            result.prelude().ok_or("prelude")?.factors()[1][0],
        )?;
    }
    if let Some(value) = values.period {
        enclosure(value, &low, &high, false)?;
        local(
            value,
            tau[0],
            result.prelude().ok_or("prelude")?.factors()[0][0],
        )?;
    }
    assert_eq!(
        result.numerical_error_scope(),
        crate::NumericalErrorScope::LocalOperation
    );
    Ok(())
}

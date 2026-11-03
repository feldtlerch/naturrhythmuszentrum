// SPDX-License-Identifier: AGPL-3.0-only
//! Known-zero inverse event points re-enter their retained original phase equation.

use super::super::support::{TestResult, integer, ledger, number};
use super::super::time::oracle::Pi;
use super::support;
use nrz_bindings::readings::algebra::syntax::Constant;
use nrz_bindings::{
    ExactConstant, LcInput, LcInterval, LcObservationValue, LcPairsCover, LcQuantity as Q,
    LcRelation, LcRelationError, LcWitness, LcZeroTimes,
};
use nrz_matrix::LcTimeProjection;
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, NumericLiteral, Scalar};

/// Both endpoints and an interior inverse event recover the same full turn on the old branch.
/// ´test:integration:lc-relation-known-zero-pointwise-roundtrip´
#[test]
fn lc_relation_known_zero_pointwise_roundtrip() -> TestResult {
    let pairs = support::cover(true, "1/20")?;
    let times = pairs.zero_times(&mut ledger()?)?;
    assert!(
        times
            .pairs_cover()
            .ok_or("retained branch")?
            .shares_allocation(&pairs)
    );
    let member = times.member(integer(3)?, &mut ledger()?)?;
    assert_eq!(member.index(), &integer(3)?);
    for (endpoint, coefficient) in member.endpoints().iter().zip(["16/5375", "48/15875"]) {
        assert_eq!(
            endpoint.coefficients(),
            (&number("0")?, &number(coefficient)?)
        );
    }
    let pi = ExactConstant::named(Constant::Pi, &mut ledger()?)?;
    let turn = pi.mul(&number("2")?.into(), &mut ledger()?)?;
    for (rate, coefficient) in [
        ("16125/8", "16/5375"),
        ("2000", "3/1000"),
        ("15875/8", "48/15875"),
    ] {
        let old = times.witness(number(rate)?, &mut ledger()?)?;
        let event = pi.mul(&number(coefficient)?.into(), &mut ledger()?)?;
        let alpha = old
            .coordinate(Q::InitialPhase)
            .ok_or("known initial phase")?;
        let lifted = alpha.add(
            &old.values().rate.mul(&event, &mut ledger()?)?,
            &mut ledger()?,
        )?;
        let recovered = lifted.div(&turn, &mut ledger()?)?;
        assert_eq!(recovered.as_rational(), Some(&integer(3)?));
        added_event(&pairs, &old, &event)?;
    }
    let oracle = Pi::new()?;
    render::<f32>(&times, &oracle)?;
    render::<f64>(&times, &oracle)
}

/// The event becomes an original observation while every old equation and constraint survives.
fn added_event(pairs: &LcPairsCover, old: &LcWitness, time: &ExactConstant) -> TestResult {
    const EVENT: usize = 3;
    let original = pairs.relation();
    let mut inputs = original.inputs().to_vec();
    for (quantity, value) in [
        (Q::Time(EVENT), time.clone()),
        (Q::Phase(EVENT), ExactConstant::zero()),
    ] {
        inputs.push(LcInput::new(
            quantity,
            &quantity.descriptor()?,
            LcInterval::exact(value, &mut ledger()?)?,
            &mut ledger()?,
        )?);
    }
    let refined = LcRelation::new(original.epoch().into(), EVENT + 1, inputs, &mut ledger()?)?;
    let inclusion = original.refinement_inputs(&refined, &mut ledger()?)?;
    assert!(inclusion.previous().shares_allocation(original));
    assert!(inclusion.refined().shares_allocation(&refined));
    assert_eq!(
        &refined.inputs()[..original.inputs().len()],
        original.inputs()
    );
    assert_eq!(inclusion.counterparts().len(), original.inputs().len());
    let mut values = old.values().clone();
    values.observations.push(LcObservationValue {
        time: time.clone(),
        phase: Some(ExactConstant::zero()),
        cycle: Some(integer(3)?),
    });
    let checked = refined.check_witness(values.clone(), &mut ledger()?)?;
    assert!(checked.relation().shares_allocation(&refined));
    assert_eq!(checked.coordinate(Q::Time(EVENT)), Some(time));
    for (before, after) in old
        .values()
        .observations
        .iter()
        .zip(&checked.values().observations)
    {
        assert_eq!(before.time, after.time);
        assert_eq!(before.phase, after.phase);
        assert_eq!(before.cycle, after.cycle);
    }
    values.observations[EVENT].cycle = Some(integer(2)?);
    assert!(matches!(
        refined.check_witness(values, &mut ledger()?),
        Err(LcRelationError::Equation)
    ));
    Ok(())
}

/// Native inverse endpoints and the original forward expression use one shared pi authority.
fn render<P: Scalar + 'static>(times: &LcZeroTimes, oracle: &Pi) -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = super::super::super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
    let native = LcTimeProjection::new(times.clone(), pi.clone(), &mut ledger()?)?;
    let member = native.member(
        integer(3)?,
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    let endpoints = member.endpoints();
    oracle.contains(endpoints[0], 16, 5375)?;
    oracle.contains(endpoints[1], 48, 15875)?;
    let interior = decode::<P>("3/1000", &context)?.mul(pi.pi().enclosure(), &context)?;
    oracle.contains(interior, 3, 1000)?;
    for (time, rate) in [
        (endpoints[0], "16125/8"),
        (interior, "2000"),
        (endpoints[1], "15875/8"),
    ] {
        let forward = decode::<P>("0", &context)?
            .add(decode::<P>(rate, &context)?.mul(time, &context)?, &context)?;
        oracle.contains(forward, 6, 1)?;
    }
    Ok(())
}

/// Fixture coefficients decode outwards; no floating center is used as an oracle.
fn decode<P: Scalar>(text: &str, context: &ArithmeticContext) -> TestResult<Enclosure<P>> {
    Ok(NumericLiteral::new(text, LiteralLimits::default())?
        .decode::<P>(context)?
        .enclosure())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Positive capacitor uncertainty retains the joint domain behind every element marginal.

use super::super::support::{TestResult, input, integer, ledger, number, pi_certificate};
use super::support;
use nrz_bindings::readings::algebra::Rational;
use nrz_bindings::{
    LcCapacitanceCover, LcCapacitanceParameters, LcDeterminacy, LcFreeDirection, LcQuantity as Q,
};
use nrz_matrix::LcCapacitanceProjection;
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, Scalar};
use std::cmp::Ordering;

/// Original phase constraints and a genuinely non-singleton capacitor interval share one question.
fn cover() -> TestResult<LcCapacitanceCover> {
    let mut inputs = support::inputs(true, "1/20")?;
    inputs.push(input(Q::Capacitance, ["9/1000000", "11/1000000"])?);
    Ok(support::relation(inputs)?.capacitance_pairs_cover(
        vec![integer(0)?, integer(0)?, integer(1)?],
        &pi_certificate()?,
        &mut ledger()?,
    )?)
}

/// Both formats enclose independent exact endpoint fractions while retaining the common joint source.
/// ´test:integration:lc-relation-capacitance-interval´
#[test]
fn lc_relation_capacitance_interval() -> TestResult {
    let exact = cover()?;
    render::<f32>(&exact)?;
    render::<f64>(&exact)
}

/// Literal expected fractions never call the production elimination formulas.
fn render<P: Scalar>(exact: &LcCapacitanceCover) -> TestResult {
    let quantities = [Q::Capacitance, Q::Inductance, Q::AmplitudeRatio, Q::Product];
    let expected = [
        ["9/1000000", "11/1000000"],
        ["4096/183051", "4096/145161"],
        ["64000/1419", "64000/1143"],
        ["64/260015625", "64/252015625"],
    ];
    let context = ArithmeticContext::new()?;
    let native = LcCapacitanceProjection::<P>::new(
        exact.clone(),
        &quantities,
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    assert!(
        native
            .cover()
            .relation()
            .shares_allocation(exact.relation())
    );
    for (quantity, endpoints) in native.quantities().iter().zip(expected) {
        assert_eq!(
            exact.determinacy(quantity.quantity())?,
            LcDeterminacy::DeterminedFinite
        );
        for (value, expected) in quantity.endpoints().iter().zip(endpoints) {
            contains(
                value.ok_or("finite capacitor projection")?,
                &number(expected)?,
            )?;
        }
    }
    for quantity in [Q::VoltageAmplitude, Q::CurrentAmplitude, Q::Energy] {
        assert_eq!(exact.determinacy(quantity)?, LcDeterminacy::Underdetermined);
        assert_eq!(
            exact.free_directions(quantity)?,
            &[LcFreeDirection::AmplitudeScale]
        );
        assert!(exact.bounds(quantity, &mut ledger()?)?.upper().is_none());
    }
    Ok(())
}

/// Exact arbitrary-precision comparisons avoid cross-product overflow in native endpoint oracles.
pub(super) fn contains<P: Scalar>(value: Enclosure<P>, expected: &Rational) -> TestResult {
    let decode = |value| -> TestResult<Rational> {
        let value = crate::kernel_oracle::Rational::binary(value)?;
        number(&format!("{}/{}", value.numerator, value.denominator))
    };
    let lower = decode(value.lower().get().to_f64())?;
    let upper = decode(value.upper().get().to_f64())?;
    assert_ne!(lower.compare(expected, &mut ledger()?)?, Ordering::Greater);
    assert_ne!(upper.compare(expected, &mut ledger()?)?, Ordering::Less);
    Ok(())
}

/// All endpoint/interior joint parameters have original witnesses; a marginal-box product does not.
/// ´test:integration:lc-relation-capacitance-joint-domain´
#[test]
fn lc_relation_capacitance_joint_domain() -> TestResult {
    let exact = cover()?;
    for rate in ["15875/8", "2000", "16125/8"] {
        for capacitance in ["9/1000000", "1/100000", "11/1000000"] {
            let parameters = parameters(rate, capacitance)?;
            let witness = exact.witness(&parameters, &mut ledger()?)?;
            assert!(witness.relation().shares_allocation(exact.relation()));
            assert_eq!(
                witness
                    .coordinate(Q::Capacitance)
                    .and_then(nrz_bindings::ExactConstant::as_rational),
                Some(&number(capacitance)?)
            );
        }
    }
    let chosen = parameters("15875/8", "11/1000000")?;
    let witness = exact.witness(&chosen, &mut ledger()?)?;
    let mut false_joint = witness.values().clone();
    false_joint.inductance = number("4096/183051")?.into();
    assert!(
        exact
            .relation()
            .check_witness(false_joint, &mut ledger()?)
            .is_err()
    );
    assert!(
        exact
            .witness(&parameters("2000", "12/1000000")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Explicit positive scale parameters, without substituting a nominal capacitor for its interval.
fn parameters(rate: &str, capacitance: &str) -> TestResult<LcCapacitanceParameters> {
    Ok(LcCapacitanceParameters {
        rate: number(rate)?,
        initial_phase: number("0")?,
        capacitance: number(capacitance)?,
        voltage: number("5")?,
        gauge: number("0")?,
    })
}

/// Unsupported extra scales and exhausted numerical work never omit an original constraint.
/// ´test:integration:lc-relation-capacitance-refusals-and-work´
#[test]
fn lc_relation_capacitance_refusals_and_work() -> TestResult {
    let mut inputs = support::inputs(true, "1/20")?;
    inputs.push(input(Q::Capacitance, ["9/1000000", "11/1000000"])?);
    inputs.push(input(Q::Inductance, ["1/40", "1/40"])?);
    assert!(
        support::relation(inputs)?
            .capacitance_pairs_cover(
                vec![integer(0)?, integer(0)?, integer(1)?],
                &pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    for range in [["0", "1/100000"], ["-1/100000", "1/100000"]] {
        let mut inputs = support::inputs(true, "1/20")?;
        inputs.push(input(Q::Capacitance, range)?);
        assert!(
            support::relation(inputs)?
                .capacitance_pairs_cover(
                    vec![integer(0)?, integer(0)?, integer(1)?],
                    &pi_certificate()?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    let exact = cover()?;
    let context = ArithmeticContext::new()?;
    let quantities = [Q::Inductance, Q::Capacitance, Q::AmplitudeRatio];
    let mut measured = ledger()?;
    LcCapacitanceProjection::<f64>::new(
        exact.clone(),
        &quantities,
        LiteralLimits::default(),
        &context,
        &mut measured,
    )?;
    let work = measured.usage().work;
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let limits = nrz_bindings::Limits::default().resolve(&nrz_bindings::LimitsOverride {
            work: Some(limit),
            ..nrz_bindings::LimitsOverride::default()
        })?;
        let mut budget = nrz_bindings::Budget::new(limits)?;
        budget.work(
            17,
            nrz_bindings::readings::algebra::Location::new("capacitor interval prefix"),
        )?;
        assert_eq!(
            LcCapacitanceProjection::<f64>::new(
                exact.clone(),
                &quantities,
                LiteralLimits::default(),
                &context,
                &mut budget
            )
            .is_ok(),
            success
        );
        assert!(budget.usage().work >= 17);
        assert_eq!(
            exact.capacitance(),
            &[number("9/1000000")?, number("11/1000000")?]
        );
    }
    Ok(())
}

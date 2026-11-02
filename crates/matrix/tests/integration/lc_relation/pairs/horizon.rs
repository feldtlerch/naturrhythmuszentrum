// SPDX-License-Identifier: AGPL-3.0-only
//! Independent rectangle and correlated-fibre thresholds keep their original clocks.

use super::super::support::{TestResult, ledger, number, pi_certificate};
use super::super::time::{self, oracle::Pi};
use super::support;
use nrz_bindings::{LcCertainty, LcHorizon, LcHorizonValue, LcQuantity as Q};
use nrz_matrix::LcHorizonProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// Independent and correlated horizons have distinct exact derivations and both-format values.
/// ´test:integration:lc-relation-forward-horizon´
#[test]
fn lc_relation_forward_horizon() -> TestResult {
    let oracle = Pi::new()?;
    let rectangle = time::cover(["5800/3", "6200/3"])?;
    let independent = rectangle.forward_horizon(&pi_certificate()?, &mut ledger()?)?;
    let correlated = support::cover(false, "1/20")?;
    let recovered = correlated.forward_horizon(&mut ledger()?)?;
    assert!(
        independent
            .relation()
            .shares_allocation(rectangle.relation())
    );
    assert!(
        recovered
            .relation()
            .shares_allocation(correlated.relation())
    );
    assert_eq!(
        independent.mathematical().width(),
        &[number("0")?, number("400/3")?]
    );
    assert_eq!(
        recovered.mathematical().width(),
        &[number("-21/110")?, number("1000/11")?]
    );
    assert_eq!(recovered.mathematical().reference(), &number("2/625")?);
    assert_eq!(
        recovered.extremal_parameters(),
        &[
            [number("21/220")?, number("21500/11")?],
            [number("-21/220")?, number("22500/11")?],
        ]
    );
    for (exact, offset, coefficient) in [
        (&independent, "0", "3/200"),
        (&recovered, "21/10000", "11/500"),
    ] {
        let LcHorizonValue::Finite(endpoint) = exact.value() else {
            return Err("finite horizon".into());
        };
        assert_eq!(
            endpoint.coefficients(),
            (&number(offset)?, &number(coefficient)?)
        );
        assert_eq!(exact.descriptor(), &Q::NaturalTime.descriptor()?);
        assert_eq!(exact.certainty(), LcCertainty::Certified);
    }
    let fixtures = [
        (&independent, "3/200", "0"),
        (&recovered, "11/500", "21/10000"),
    ];
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)?;
    Ok(())
}

/// Scalar pi is prepared once per format and retained with its separate duration error.
fn render<P: Scalar + 'static>(fixtures: &[(&LcHorizon, &str, &str)], oracle: &Pi) -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = super::super::super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
    for &(exact, coefficient, offset) in fixtures {
        let native = LcHorizonProjection::new(
            exact.clone(),
            pi.clone(),
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert!(native.pi().same_region(&pi));
        oracle.affine(
            native.enclosure().ok_or("finite enclosure")?,
            &number(coefficient)?,
            &number(offset)?,
        )?;
    }
    Ok(())
}

/// A singleton frequency has an explicit infinite horizon in both formats, with no fake endpoint.
/// ´test:integration:lc-relation-infinite-horizon´
#[test]
fn lc_relation_infinite_horizon() -> TestResult {
    let exact =
        time::cover(["2000", "2000"])?.forward_horizon(&pi_certificate()?, &mut ledger()?)?;
    assert!(matches!(exact.value(), LcHorizonValue::Infinite));
    infinite::<f32>(&exact)?;
    infinite::<f64>(&exact)
}

/// Infinite mathematical values stay absent native endpoints, retaining the exact outcome.
fn infinite<P: Scalar + 'static>(exact: &LcHorizon) -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = super::super::super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
    let native = LcHorizonProjection::new(
        exact.clone(),
        pi,
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    assert!(native.enclosure().is_none());
    assert!(matches!(native.exact().value(), LcHorizonValue::Infinite));
    Ok(())
}

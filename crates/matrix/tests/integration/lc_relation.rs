// SPDX-License-Identifier: AGPL-3.0-only
//! Both-format forward boxes use independently specified rational and enclosed-pi oracles.

mod pairs;
mod refusals;
mod support;
mod time;

use crate::kernel_oracle::Rational as Oracle;
use nrz_bindings::{LcCertainty, LcDeterminacy, LcQuantity as Q};
use nrz_matrix::LcForwardProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};
use support::{TestResult, cover, ledger, number};

/// Exact element/time supports retain all wrapped arcs and the excluded gap in both formats.
/// ´test:integration:lc-relation-forward-input-boxes´
#[test]
fn lc_relation_forward_input_boxes() -> TestResult {
    forward::<f32>()?;
    forward::<f64>()
}

/// Independent fixture endpoints never call the production projection formula.
fn forward<P: Scalar + 'static>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let pi = super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
    for (times, indices, expected) in [
        (["1/4", "1/4"], [0, 0], [(1, 4), (1, 1)]),
        (["1/4", "1/2"], [0, 0], [(1, 4), (2, 1)]),
    ] {
        let exact = cover(times, indices)?;
        assert_eq!(exact.determinacy(), LcDeterminacy::DeterminedFinite);
        assert_eq!(exact.certainty(), LcCertainty::Certified);
        let native = LcForwardProjection::new(
            exact,
            pi.clone(),
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert!(native.pi().same_region(&pi));
        assert_eq!(native.arcs().len(), 1);
        let arc = &native.arcs()[0];
        for (endpoint, (top, bottom)) in arc.endpoints().iter().zip(expected) {
            support::contains(*endpoint, Oracle::new(top, bottom)?)?;
        }
        assert!(arc.exact().upper_included());
        assert_eq!(arc.exact().index(), &number("0")?);
    }
    let exact = cover(["2", "2"], [0, 1])?;
    assert_eq!(exact.lifted(), &[number("2")?, number("8")?]);
    let native = LcForwardProjection::new(
        exact,
        pi,
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    assert_eq!(native.arcs().len(), 2);
    let [first, second] = native.arcs() else {
        return Err("two complete wrap arcs".into());
    };
    assert!(!first.exact().upper_included());
    assert!(second.exact().upper_included());
    support::contains(first.endpoints()[0], Oracle::new(2, 1)?)?;
    support::contains(second.endpoints()[0], Oracle::new(0, 1)?)?;
    for endpoint in super::pi_conversion::oracle::factor(2, 1, false, 52)? {
        support::contains(first.endpoints()[1], endpoint)?;
        support::contains(
            second.endpoints()[1],
            Oracle::new(8, 1)?.subtract(endpoint)?,
        )?;
    }
    let gap = Oracle::new(19, 10)?;
    assert!(Oracle::binary(second.enclosure().upper().get().to_f64())?.at_most(gap)?);
    assert!(gap.at_most(Oracle::binary(first.enclosure().lower().get().to_f64())?)?);
    Ok(())
}

/// Interior and endpoint parameter instances are checked against every original LC equation.
/// ´test:integration:lc-relation-forward-parametric-witnesses´
#[test]
fn lc_relation_forward_parametric_witnesses() -> TestResult {
    let exact = cover(["1/4", "1/2"], [0, 0])?;
    for rate in ["1", "5/4", "2", "3", "4"] {
        let point = exact.witness(number(rate)?, number("0")?, &mut ledger()?)?;
        assert!(point.relation().shares_allocation(exact.relation()));
        assert_eq!(
            point
                .coordinate(Q::Rate)
                .and_then(nrz_bindings::ExactConstant::as_rational),
            Some(&number(rate)?)
        );
        assert_eq!(
            point
                .coordinate(Q::Energy)
                .and_then(nrz_bindings::ExactConstant::as_rational),
            Some(&number("1")?)
        );
    }
    Ok(())
}

/// Complete index ranges survive materialization and signed initial-phase lifts.
/// ´test:integration:lc-relation-forward-complete-cycle-range´
#[test]
fn lc_relation_forward_complete_cycle_range() -> TestResult {
    let exact = cover(["20", "20"], [3, 12])?;
    assert_eq!(exact.cycles(), &[number("3")?, number("12")?]);
    for index in 3..=12 {
        let integer = support::integer(index)?;
        let arc = exact.arc(integer.clone(), &mut ledger()?)?;
        assert_eq!(arc.index(), &integer);
        assert_eq!(arc.upper_included(), index == 12);
    }
    let mut inputs = support::inputs()?;
    inputs.retain(|input| input.quantity() != Q::InitialPhase);
    inputs.push(support::input(Q::InitialPhase, ["-1", "0"])?);
    let original = support::relation(inputs)?;
    let exact = original.forward_cover(
        support::input(Q::Time(0), ["0", "0"])?,
        [number("-1")?, number("0")?],
        &support::pi_certificate()?,
        &mut ledger()?,
    )?;
    assert_eq!(exact.cycles(), &[number("-1")?, number("0")?]);
    let final_arc = exact.arc(number("0")?, &mut ledger()?)?;
    assert_eq!(
        final_arc.endpoints().0.coefficients(),
        (&number("0")?, &number("0")?)
    );
    assert!(final_arc.upper_included());
    Ok(())
}

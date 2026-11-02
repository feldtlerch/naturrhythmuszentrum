// SPDX-License-Identifier: AGPL-3.0-only
//! Unsupported constraints and resource limits cannot publish a narrowed or partial family.

use super::support::{self, TestResult, cover, input, ledger, number, relation};
use nrz_bindings::{Budget, LcQuantity as Q, LcRelationError, Limits, LimitsOverride};
use nrz_matrix::LcForwardProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits};

/// Missing cycle endpoints, nonintegers and false pi evidence refuse the entire cover.
/// ´test:integration:lc-relation-forward-cover-refusals´
#[test]
fn lc_relation_forward_cover_refusals() -> TestResult {
    for indices in [[0, 0], [1, 1], [-1, 1], [1, 0]] {
        assert!(cover(["2", "2"], indices).is_err());
    }
    assert!(cover(["20", "20"], [3, 11]).is_err());
    let original = relation(support::inputs()?)?;
    let mut pi = support::pi_certificate()?;
    pi.upper = number("3")?;
    assert!(
        original
            .forward_cover(
                input(Q::Time(0), ["2", "2"])?,
                [number("0")?, number("1")?],
                &pi,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(
        original
            .forward_cover(
                input(Q::Time(0), ["2", "2"])?,
                [number("0")?, number("1/2")?],
                &support::pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(
        cover(["2", "2"], [0, 1])?
            .arc(number("2")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Extra constraints and unsupported domains are never ignored to recover a preferred answer.
/// ´test:integration:lc-relation-forward-constraint-refusals´
#[test]
fn lc_relation_forward_constraint_refusals() -> TestResult {
    for extra in [
        input(Q::AmplitudeRatio, ["50", "50"])?,
        input(Q::Energy, ["0", "0"])?,
        input(Q::Inductance, ["2", "3"])?,
    ] {
        let mut inputs = support::inputs()?;
        inputs.push(extra);
        let original = relation(inputs)?;
        assert!(matches!(
            original.forward_cover(
                input(Q::Time(0), ["1/4", "1/4"])?,
                [number("0")?, number("0")?],
                &support::pi_certificate()?,
                &mut ledger()?
            ),
            Err(LcRelationError::Projection)
        ));
    }
    assert!(cover(["-1", "1"], [0, 0]).is_err());
    let exact = cover(["2", "2"], [0, 1])?;
    assert!(
        exact
            .witness(number("5")?, number("0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        exact
            .witness(number("2")?, number("1")?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// A failed numerical allocation retains the exact complete cover and consumes the caller ledger.
/// ´test:integration:lc-relation-forward-numerical-resource-refusals´
#[test]
fn lc_relation_forward_numerical_resource_refusals() -> TestResult {
    let exact = cover(["2", "2"], [0, 1])?;
    let context = ArithmeticContext::new()?;
    let pi = super::super::pi_conversion::pi::<f64>(&context, &mut ledger()?)?;
    let limits = Limits::default().resolve(&LimitsOverride {
        output_bytes: Some(1),
        ..LimitsOverride::default()
    })?;
    let mut budget = Budget::new(limits)?;
    assert!(
        LcForwardProjection::new(
            exact.clone(),
            pi.clone(),
            LiteralLimits::default(),
            &context,
            &mut budget
        )
        .is_err()
    );
    assert!(budget.usage().work > 0);
    assert_eq!(exact.cycles(), &[number("0")?, number("1")?]);
    let mut measured = ledger()?;
    let native = LcForwardProjection::new(
        exact.clone(),
        pi.clone(),
        LiteralLimits::default(),
        &context,
        &mut measured,
    )?;
    let work = measured.usage().work;
    assert_eq!(native.usage()[1].work - native.usage()[0].work, work);
    for (limit, expected) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(
            17,
            nrz_bindings::readings::algebra::Location::new("LC projection prefix"),
        )?;
        assert_eq!(
            LcForwardProjection::new(
                exact.clone(),
                pi.clone(),
                LiteralLimits::default(),
                &context,
                &mut budget
            )
            .is_ok(),
            expected
        );
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

/// A valid but coarse pi certificate cannot settle the high-cycle upper face.
/// ´test:integration:lc-relation-forward-coarse-pi-refusal´
#[test]
fn lc_relation_forward_coarse_pi_refusal() -> TestResult {
    let original = relation(support::inputs()?)?;
    let mut pi = support::pi_certificate()?;
    pi.lower = number("3")?;
    assert!(
        original
            .forward_cover(
                input(Q::Time(0), ["20", "20"])?,
                [number("3")?, number("12")?],
                &pi,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(cover(["20", "20"], [3, 12]).is_ok());
    Ok(())
}

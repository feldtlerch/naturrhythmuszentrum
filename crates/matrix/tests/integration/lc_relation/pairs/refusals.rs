// SPDX-License-Identifier: AGPL-3.0-only
//! Missing alias bounds, hidden constraints and resource exhaustion cannot certify a preferred branch.

use super::super::support::{TestResult, input, integer, ledger, number, pi_certificate};
use super::support;
use nrz_bindings::{Budget, LcPairParameters, LcQuantity as Q, Limits, LimitsOverride};
use nrz_matrix::LcPairsProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits};

/// A bounded-branch certificate must exclude all omitted relative cycle possibilities.
/// ´test:integration:lc-relation-recovery-cover-refusals´
#[test]
fn lc_relation_recovery_cover_refusals() -> TestResult {
    for replacement in [None, Some(["1", "100000"])] {
        let mut inputs = support::inputs(false, "1/20")?;
        inputs.retain(|input| input.quantity() != Q::AngularFrequency);
        if let Some(bounds) = replacement {
            inputs.push(input(Q::AngularFrequency, bounds)?);
        }
        let relation = support::relation(inputs)?;
        assert!(
            relation
                .pairs_cover(
                    vec![integer(0)?, integer(0)?, integer(1)?],
                    &pi_certificate()?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    let original = support::relation(support::inputs(false, "1/20")?)?;
    assert!(
        original
            .pairs_cover(
                vec![integer(0)?, integer(1)?, integer(1)?],
                &pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(
        original
            .pairs_cover(
                vec![number("1/2")?, integer(0)?, integer(1)?],
                &pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    Ok(())
}

/// Unsupported extra quantities, uncertain times and a false phase/rate pairing refuse.
/// ´test:integration:lc-relation-recovery-constraint-refusals´
#[test]
fn lc_relation_recovery_constraint_refusals() -> TestResult {
    let mut inputs = support::inputs(false, "1/20")?;
    inputs.push(input(Q::Product, ["1", "1"])?);
    assert!(
        support::relation(inputs)?
            .pairs_cover(
                vec![integer(0)?, integer(0)?, integer(1)?],
                &pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    let mut inputs = support::inputs(false, "1/20")?;
    inputs.retain(|input| input.quantity() != Q::Time(1));
    inputs.push(input(Q::Time(1), ["1/500", "3/1000"])?);
    assert!(
        support::relation(inputs)?
            .pairs_cover(
                vec![integer(0)?, integer(0)?, integer(1)?],
                &pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    let exact = support::cover(false, "1/20")?;
    let wrong = LcPairParameters {
        rate: number("22500/11")?,
        initial_phase: number("0")?,
        ratio: number("50")?,
        voltage: number("5")?,
        gauge: number("0")?,
    };
    assert!(exact.witness(&wrong, &mut ledger()?).is_err());
    let fixed = support::cover(true, "1/20")?;
    let wrong = LcPairParameters {
        rate: number("2000")?,
        gauge: number("1")?,
        ..wrong
    };
    assert!(fixed.witness(&wrong, &mut ledger()?).is_err());
    Ok(())
}

/// Numerical publication is all-or-nothing and reconciles failed decoding on the same ledger.
/// ´test:integration:lc-relation-recovery-cumulative-work´
#[test]
fn lc_relation_recovery_cumulative_work() -> TestResult {
    let exact = support::cover(true, "1/20")?;
    let context = ArithmeticContext::new()?;
    let mut measured = ledger()?;
    let native = LcPairsProjection::<f64>::new(
        exact.clone(),
        &[Q::Rate, Q::Product],
        LiteralLimits::default(),
        &context,
        &mut measured,
    )?;
    let work = measured.usage().work;
    assert_eq!(native.usage()[1].work - native.usage()[0].work, work);
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(
            17,
            nrz_bindings::readings::algebra::Location::new("LC pairs prefix"),
        )?;
        assert_eq!(
            LcPairsProjection::<f64>::new(
                exact.clone(),
                &[Q::Rate, Q::Product],
                LiteralLimits::default(),
                &context,
                &mut budget
            )
            .is_ok(),
            success
        );
        assert!(budget.usage().work >= 17);
    }
    Ok(())
}

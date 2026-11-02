// SPDX-License-Identifier: AGPL-3.0-only
//! Affine-width thresholds and exhausted numerical lowering preserve their exact questions.

use super::super::support::{TestResult, ledger, number, pi_certificate};
use super::super::time;
use nrz_bindings::readings::algebra::lattice::{PiHorizonValue, PiWidthHorizon};
use nrz_bindings::{Budget, Limits, LimitsOverride};
use nrz_matrix::LcHorizonProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits};

/// Coarse constant evidence is explicitly undecided, while full and infinite cases remain distinct.
/// ´test:integration:lc-relation-horizon-domain-outcomes´
#[test]
fn lc_relation_horizon_domain_outcomes() -> TestResult {
    let pi = pi_certificate()?.check(&mut ledger()?)?;
    let near = PiWidthHorizon::check(
        [number("25/4")?, number("0")?],
        number("0")?,
        pi.clone(),
        &mut ledger()?,
    )?;
    assert!(matches!(near.value(), PiHorizonValue::Undecided));
    assert_eq!(near.width(), &[number("25/4")?, number("0")?]);
    let full = PiWidthHorizon::check(
        [number("7")?, number("0")?],
        number("3")?,
        pi.clone(),
        &mut ledger()?,
    )?;
    let PiHorizonValue::Finite(value) = full.value() else {
        return Err("full at future reference".into());
    };
    assert_eq!(value.coefficients(), (&number("3")?, &number("0")?));
    for width in [["0", "-1"], ["-1", "0"]] {
        assert!(
            PiWidthHorizon::check(
                [number(width[0])?, number(width[1])?],
                number("0")?,
                pi.clone(),
                &mut ledger()?
            )
            .is_err()
        );
    }
    Ok(())
}

/// A failed numerical horizon attempt keeps its independent exact duration and original relation.
/// ´test:integration:lc-relation-horizon-cumulative-work´
#[test]
fn lc_relation_horizon_cumulative_work() -> TestResult {
    let source = time::cover(["5800/3", "6200/3"])?;
    let exact = source.forward_horizon(&pi_certificate()?, &mut ledger()?)?;
    let context = ArithmeticContext::new()?;
    let pi = super::super::super::pi_conversion::pi::<f64>(&context, &mut ledger()?)?;
    let mut measured = ledger()?;
    let native = LcHorizonProjection::new(
        exact.clone(),
        pi.clone(),
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
            nrz_bindings::readings::algebra::Location::new("horizon prefix"),
        )?;
        assert_eq!(
            LcHorizonProjection::new(
                exact.clone(),
                pi.clone(),
                LiteralLimits::default(),
                &context,
                &mut budget
            )
            .is_ok(),
            success
        );
        assert!(budget.usage().work >= 17);
        assert!(exact.relation().shares_allocation(source.relation()));
    }
    Ok(())
}

/// Original-equation witnesses realize both correlated future extremizers on the retained branch.
/// ´test:integration:lc-relation-correlated-horizon-extrema´
#[test]
fn lc_relation_correlated_horizon_extrema() -> TestResult {
    let exact = super::support::cover(false, "1/20")?;
    let horizon = exact.forward_horizon(&mut ledger()?)?;
    assert_eq!(horizon.mathematical().initial_width(), &number("1/10")?);
    for point in horizon.extremal_parameters() {
        let parameters = nrz_bindings::LcPairParameters {
            initial_phase: point[0].clone(),
            rate: point[1].clone(),
            ratio: number("50")?,
            voltage: number("5")?,
            gauge: number("0")?,
        };
        let witness = exact.witness(&parameters, &mut ledger()?)?;
        assert!(witness.relation().shares_allocation(horizon.relation()));
    }
    for time in ["2/625", "1/100", "1/10"] {
        let time = number(time)?;
        let endpoints = horizon.extremal_parameters();
        let lower =
            endpoints[0][0].add(&endpoints[0][1].mul(&time, &mut ledger()?)?, &mut ledger()?)?;
        let upper =
            endpoints[1][0].add(&endpoints[1][1].mul(&time, &mut ledger()?)?, &mut ledger()?)?;
        for rate in ["21500/11", "1975", "2000", "2025", "22500/11"] {
            let rate = number(rate)?;
            for alpha in exact.phase_at_rate(&rate, &mut ledger()?)? {
                let lifted = alpha.add(&rate.mul(&time, &mut ledger()?)?, &mut ledger()?)?;
                assert_ne!(
                    lifted.compare(&lower, &mut ledger()?)?,
                    std::cmp::Ordering::Less
                );
                assert_ne!(
                    lifted.compare(&upper, &mut ledger()?)?,
                    std::cmp::Ordering::Greater
                );
            }
        }
    }
    Ok(())
}

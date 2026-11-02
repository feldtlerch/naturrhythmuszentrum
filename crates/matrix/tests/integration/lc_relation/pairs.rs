// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact recovery intervals retain unknown-phase correlation and free scale.

mod aliases;
mod attempt;
mod capacitance;
mod clock;
mod horizon;
mod horizon_checks;
mod impossible;
mod refinement;
mod refusals;
mod scale;
mod support;

use super::support::{TestResult, contains, ledger, number};
use crate::kernel_oracle::Rational as Oracle;
use nrz_bindings::{
    LcCertainty, LcDeterminacy, LcFreeDirection, LcPairParameters, LcPhaseGauge, LcQuantity as Q,
};
use nrz_matrix::LcPairsProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// Unknown initial phase is recovered jointly rather than replaced by its generating value.
/// ´test:integration:lc-relation-recovery´
#[test]
fn lc_relation_recovery() -> TestResult {
    recovered::<f32>(
        false,
        [(21500, 11), (22500, 11)],
        [(121, 506_250_000), (121, 462_250_000)],
    )?;
    recovered::<f64>(
        false,
        [(21500, 11), (22500, 11)],
        [(121, 506_250_000), (121, 462_250_000)],
    )
}

/// Supplying exact initial phase tightens the complete correlated rate and product ranges.
/// ´test:integration:lc-relation-known-phase-recovery´
#[test]
fn lc_relation_known_phase_recovery() -> TestResult {
    recovered::<f32>(
        true,
        [(15875, 8), (16125, 8)],
        [(64, 260_015_625), (64, 252_015_625)],
    )?;
    recovered::<f64>(
        true,
        [(15875, 8), (16125, 8)],
        [(64, 260_015_625), (64, 252_015_625)],
    )
}

/// Rational expected endpoints are independent of every production elimination operation.
fn recovered<P: Scalar>(
    known: bool,
    rate: [(i128, i128); 2],
    product: [(i128, i128); 2],
) -> TestResult {
    let exact = support::cover(known, "1/20")?;
    assert_eq!(exact.certainty(), LcCertainty::Certified);
    assert_eq!(
        exact.determinacy(Q::Product)?,
        LcDeterminacy::DeterminedFinite
    );
    assert_eq!(
        exact.determinacy(Q::Inductance)?,
        LcDeterminacy::Underdetermined
    );
    assert_eq!(
        exact.free_directions(Q::Inductance)?,
        &[LcFreeDirection::ElementScale]
    );
    assert_eq!(
        exact.phase_gauge(),
        if known {
            LcPhaseGauge::Fixed
        } else {
            LcPhaseGauge::AllIntegers
        }
    );
    assert!(
        exact
            .feasibility_witness()
            .relation()
            .shares_allocation(exact.relation())
    );
    let context = ArithmeticContext::new()?;
    let native = LcPairsProjection::<P>::new(
        exact,
        &[Q::Rate, Q::Product, Q::Inductance, Q::Capacitance],
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    for (quantity, expected) in native.quantities().iter().take(2).zip([rate, product]) {
        for (endpoint, (top, bottom)) in quantity.endpoints().iter().zip(expected) {
            contains(
                endpoint.ok_or("finite endpoint")?,
                Oracle::new(top, bottom)?,
            )?;
        }
    }
    for quantity in &native.quantities()[2..] {
        assert_eq!(quantity.endpoints()[1], None);
        assert_eq!(quantity.exact().upper(), None);
    }
    Ok(())
}

/// Every recovered rate has its own feasible phase fibre and arbitrary positive element scale.
/// ´test:integration:lc-relation-recovery-parametric-witnesses´
#[test]
fn lc_relation_recovery_parametric_witnesses() -> TestResult {
    for known in [false, true] {
        let exact = support::cover(known, "1/20")?;
        for rate in [&exact.rate()[0], &number("2000")?, &exact.rate()[1]] {
            let phase = exact.phase_at_rate(rate, &mut ledger()?)?;
            for ratio in ["1/50", "1", "50", "10000"] {
                let parameters = LcPairParameters {
                    rate: rate.clone(),
                    initial_phase: phase[0].clone(),
                    ratio: number(ratio)?,
                    voltage: number("5")?,
                    gauge: number("0")?,
                };
                let witness = exact.witness(&parameters, &mut ledger()?)?;
                assert!(witness.relation().shares_allocation(exact.relation()));
            }
        }
    }
    let exact = support::cover(false, "1/20")?;
    let phase = exact.phase_at_rate(&number("22500/11")?, &mut ledger()?)?;
    assert_eq!(phase, [number("-21/220")?, number("-21/220")?]);
    let parameters = LcPairParameters {
        rate: number("2000")?,
        initial_phase: number("0")?,
        ratio: number("50")?,
        voltage: number("5")?,
        gauge: number("-9")?,
    };
    let witness = exact.witness(&parameters, &mut ledger()?)?;
    for (observation, cycle) in witness.values().observations.iter().zip([-9, -9, -8]) {
        assert_eq!(
            observation.cycle.as_ref(),
            Some(&super::support::integer(cycle)?)
        );
    }
    Ok(())
}

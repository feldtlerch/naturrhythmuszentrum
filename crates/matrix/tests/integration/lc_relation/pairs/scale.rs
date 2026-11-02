// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact element, amplitude and energy fixtures share the recovered phase relation.

use super::super::support::{TestResult, contains, input, integer, ledger, number, pi_certificate};
use super::support;
use crate::kernel_oracle::Rational as Oracle;
use nrz_bindings::{
    LcCertainty, LcConsistency, LcDeterminacy, LcFreeDirection, LcPairParameters, LcPairsCover,
    LcQuantity as Q,
};
use nrz_matrix::LcPairsProjection;
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// Either known element determines the other on the same complete recovered rate/product branch.
/// ´test:integration:lc-relation-known-element´
#[test]
fn lc_relation_known_element() -> TestResult {
    known_element::<f32>()?;
    known_element::<f64>()
}

/// Expected endpoint fractions are independent of the production eliminations.
fn known_element<P: Scalar>() -> TestResult {
    for (known, value, query, expected) in [
        (
            Q::Capacitance,
            "1/100000",
            Q::Inductance,
            [(2048, 83205), (2048, 80645)],
        ),
        (
            Q::Inductance,
            "1/40",
            Q::Capacitance,
            [(512, 52_003_125), (512, 50_403_125)],
        ),
    ] {
        let exact = cover(&[(known, value)])?;
        assert_eq!(exact.determinacy(query)?, LcDeterminacy::DeterminedFinite);
        let context = ArithmeticContext::new()?;
        let native = LcPairsProjection::<P>::new(
            exact,
            &[query],
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        for (endpoint, (top, bottom)) in native.quantities()[0].endpoints().iter().zip(expected) {
            contains(endpoint.ok_or("finite element")?, Oracle::new(top, bottom)?)?;
        }
    }
    Ok(())
}

/// Ratio and amplitude/energy eliminations retain scale freedom exactly when original inputs do.
/// ´test:integration:lc-relation-amplitude-energy´
#[test]
fn lc_relation_amplitude_energy() -> TestResult {
    amplitude_energy::<f32>()?;
    amplitude_energy::<f64>()
}

/// Both formats use independent endpoints and exact full-tuple consistency evidence.
fn amplitude_energy<P: Scalar>() -> TestResult {
    ratio::<P>()?;
    energy_voltage::<P>()?;
    consistency()
}

/// A supplied ratio fixes both positive elements on the same rate branch.
fn ratio<P: Scalar>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let ratio = cover(&[(Q::AmplitudeRatio, "50")])?;
    let native = LcPairsProjection::<P>::new(
        ratio,
        &[Q::Inductance, Q::Capacitance],
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    for (quantity, expected) in native
        .quantities()
        .iter()
        .zip([[(16, 645), (16, 635)], [(4, 403_125), (4, 396_875)]])
    {
        for (endpoint, (top, bottom)) in quantity.endpoints().iter().zip(expected) {
            contains(
                endpoint.ok_or("finite ratio-derived element")?,
                Oracle::new(top, bottom)?,
            )?;
        }
    }
    Ok(())
}

/// Energy and voltage eliminate capacitance before projecting the original product.
fn energy_voltage<P: Scalar>() -> TestResult {
    let context = ArithmeticContext::new()?;
    let energy_voltage = cover(&[(Q::Energy, "1/8000"), (Q::VoltageAmplitude, "5")])?;
    let native = LcPairsProjection::<P>::new(
        energy_voltage,
        &[Q::Capacitance, Q::Inductance],
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    for endpoint in native.quantities()[0].endpoints() {
        contains(
            endpoint.ok_or("fixed capacitance")?,
            Oracle::new(1, 100_000)?,
        )?;
    }
    for (endpoint, (top, bottom)) in native.quantities()[1]
        .endpoints()
        .iter()
        .zip([(2048, 83205), (2048, 80645)])
    {
        contains(
            endpoint.ok_or("energy-derived element")?,
            Oracle::new(top, bottom)?,
        )?;
    }
    Ok(())
}

/// Energy alone retains free scale; the complete supplied tuple proves joint consistency.
fn consistency() -> TestResult {
    let energy = cover(&[(Q::Energy, "1/8000")])?;
    assert_eq!(energy.certainty(), LcCertainty::Certified);
    assert_eq!(
        energy.determinacy(Q::Inductance)?,
        LcDeterminacy::Underdetermined
    );
    assert_eq!(
        energy.free_directions(Q::Inductance)?,
        &[LcFreeDirection::ElementScale]
    );
    assert_eq!(energy.bounds(Q::Inductance, &mut ledger()?)?.upper(), None);
    let full = cover(&[
        (Q::Inductance, "1/40"),
        (Q::Capacitance, "1/100000"),
        (Q::VoltageAmplitude, "5"),
        (Q::CurrentAmplitude, "1/10"),
        (Q::Energy, "1/8000"),
    ])?;
    assert_eq!(full.consistency(), LcConsistency::Overdetermined);
    assert_eq!(full.rate(), &[number("2000")?, number("2000")?]);
    assert_eq!(
        full.feasibility_witness()
            .coordinate(Q::AmplitudeRatio)
            .and_then(nrz_bindings::ExactConstant::as_rational),
        Some(&number("50")?)
    );
    Ok(())
}

/// Fixed energy admits arbitrary positive impedance scales with exact root amplitudes.
/// ´test:integration:lc-relation-energy-free-scale-witnesses´
#[test]
fn lc_relation_energy_free_scale_witnesses() -> TestResult {
    let exact = cover(&[(Q::Energy, "1/8000")])?;
    for rate in [&exact.rate()[0], &number("2000")?, &exact.rate()[1]] {
        for ratio in ["1/50", "1", "50", "10000"] {
            let parameters = LcPairParameters {
                rate: rate.clone(),
                initial_phase: number("0")?,
                ratio: number(ratio)?,
                voltage: number("1")?,
                gauge: number("0")?,
            };
            let witness = exact.witness(&parameters, &mut ledger()?)?;
            assert_eq!(
                witness
                    .coordinate(Q::Energy)
                    .and_then(nrz_bindings::ExactConstant::as_rational),
                Some(&number("1/8000")?)
            );
        }
    }
    assert!(
        cover(&[
            (Q::Capacitance, "1/100000"),
            (Q::Energy, "1/4000"),
            (Q::VoltageAmplitude, "5")
        ])
        .is_err()
    );
    Ok(())
}

/// Every scale constraint remains supplied on the same original pair relation.
fn cover(scales: &[(Q, &str)]) -> TestResult<LcPairsCover> {
    let mut inputs = support::inputs(true, "1/20")?;
    for (quantity, value) in scales {
        inputs.push(input(*quantity, [value, value])?);
    }
    Ok(support::relation(inputs)?.pairs_cover(
        vec![integer(0)?, integer(0)?, integer(1)?],
        &pi_certificate()?,
        &mut ledger()?,
    )?)
}

/// Ratio and energy are queryable outputs of supplied physical scales in both formats.
/// ´test:integration:lc-relation-query-ratio-energy´
#[test]
fn lc_relation_query_ratio_energy() -> TestResult {
    queried_scales::<f32>()?;
    queried_scales::<f64>()
}

/// A supplied capacitor and voltage determine energy and a correlated ratio/current interval.
fn queried_scales<P: Scalar>() -> TestResult {
    let exact = cover(&[(Q::Capacitance, "1/100000"), (Q::VoltageAmplitude, "5")])?;
    let context = ArithmeticContext::new()?;
    let native = LcPairsProjection::<P>::new(
        exact,
        &[Q::Energy, Q::AmplitudeRatio, Q::CurrentAmplitude],
        LiteralLimits::default(),
        &context,
        &mut ledger()?,
    )?;
    for (quantity, expected) in native.quantities().iter().zip([
        [(1, 8000), (1, 8000)],
        [(6400, 129), (6400, 127)],
        [(127, 1280), (129, 1280)],
    ]) {
        assert_eq!(
            native.cover().determinacy(quantity.quantity())?,
            LcDeterminacy::DeterminedFinite
        );
        for (endpoint, (top, bottom)) in quantity.endpoints().iter().zip(expected) {
            contains(
                endpoint.ok_or("finite derived scale")?,
                Oracle::new(top, bottom)?,
            )?;
        }
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Independent rational LC derivatives and atan bounds distinguish both covariance axes and targets.

use super::{Prepared, TestResult, models, prepared, publication, shared};
use crate::integration::pi_conversion::oracle;
use crate::reciprocal_testing::oracle::Rational;
use nrz_matrix::{CovarianceBlock, LcAngularFrequencyParameter};
use nrz_scalar::{ArithmeticContext, LiteralLimits, NumericLiteral, Scalar};

/// Reference and varied original represented centers retain original-law derivatives in both formats.
/// ´test:integration:lc-frequency-original-center-covariance-oracles´
#[test]
fn lc_frequency_original_center_covariance_oracles() -> TestResult {
    let prepared = prepared()?;
    original::<f32>(&prepared)?;
    original::<f64>(&prepared)
}

fn original<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    for (point, coefficients) in crate::omega_oracle::cases::<P>() {
        let mut budget = shared::policy()?;
        let mut case = models::case(
            prepared,
            point.map(|value| [value, P::ZERO, P::ZERO]),
            [coefficients[0], coefficients[1], P::ZERO, coefficients[1]],
            [P::ONE, coefficients[2]],
            CovarianceBlock::Known,
            &mut budget,
        )?;
        let context = ArithmeticContext::new()?;
        let plan = publication(prepared, &case, &mut budget)?;
        let output = case
            .heap
            .publish_lc_frequency(&plan, &context, &mut budget)?;
        let (squares, variance) = crate::omega_oracle::targets(point, coefficients, None)?;
        let covariance = output.covariance(&case.heap, &context)?;
        assert!(variance.covered(
            covariance[0][0].center.to_f64(),
            covariance[0][0].error.to_f64()
        )?);
        scaled_covariance(covariance, &variance)?;
        let jacobians = output.jacobians(&case.heap, &context)?;
        for (actual, square) in jacobians[0].into_iter().zip(&squares) {
            crate::omega_oracle::negative_square(
                actual.center.to_f64(),
                actual.error.to_f64(),
                square,
            )?;
        }
        for endpoint in oracle::factor(1, 2, true, 56)? {
            let factor = big(endpoint)?;
            let coefficient_power = factor.multiply(&factor)?;
            for (actual, square) in jacobians[1].into_iter().zip(&squares) {
                crate::omega_oracle::negative_square(
                    actual.center.to_f64(),
                    actual.error.to_f64(),
                    &square.multiply(&coefficient_power)?,
                )?;
            }
        }
        let value = output.values(&case.heap, &context)?[1];
        assert!(value.center > P::ZERO);
        assert!(!Rational::new(-1, 1)?.covered(value.center.to_f64(), value.total.to_f64())?);
        assert_eq!(output.operating_point(&case.heap, &context)?, point);
    }
    Ok(())
}

/// Both covariance axes use the coefficient, while either cross axis uses it exactly once.
pub(super) fn scaled_covariance<P: Scalar>(
    actual: [[LcAngularFrequencyParameter<P>; 2]; 2],
    variance: &Rational,
) -> TestResult {
    for endpoint in oracle::factor(1, 2, true, 56)? {
        let factor = big(endpoint)?;
        for parameter in [actual[0][1], actual[1][0]] {
            assert!(
                variance
                    .multiply(&factor)?
                    .covered(parameter.center.to_f64(), parameter.error.to_f64())?
            );
        }
        assert!(
            variance
                .multiply(&factor.multiply(&factor)?)?
                .covered(actual[1][1].center.to_f64(), actual[1][1].error.to_f64())?
        );
    }
    Ok(())
}

/// Nominal decimal inputs carry their certified representation error separately from the original point.
/// ´test:integration:lc-frequency-reference-and-period-oracle´
#[test]
fn lc_frequency_reference_and_period_oracle() -> TestResult {
    let prepared = prepared()?;
    reference::<f32>(&prepared)?;
    reference::<f64>(&prepared)
}

fn reference<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let context = ArithmeticContext::new()?;
    let mut values = [[P::ZERO; 3]; 2];
    for (slot, literal) in values.iter_mut().zip(["1/40", "1/100000"]) {
        let decoded =
            NumericLiteral::new(literal, LiteralLimits::default())?.decode::<P>(&context)?;
        *slot = [
            decoded.center().get(),
            decoded.displacement().get(),
            decoded.displacement().get(),
        ];
    }
    let mut budget = shared::policy()?;
    let mut case = models::case(
        prepared,
        values,
        [P::ZERO; 4],
        [P::ONE; 2],
        CovarianceBlock::DeclaredZero("fixed nominal frequency reference".into()),
        &mut budget,
    )?;
    let plan = publication(prepared, &case, &mut budget)?;
    let output = case
        .heap
        .publish_lc_frequency(&plan, &context, &mut budget)?;
    let frequency = output.values(&case.heap, &context)?[1];
    oracle::covers(
        oracle::factor(1000, 1, true, 48)?,
        frequency.center.to_f64(),
        frequency.error.to_f64(),
    )?;
    let lower = Rational::binary(frequency.center.to_f64())?
        .add(&Rational::binary(-frequency.error.to_f64())?)?;
    let upper = Rational::binary(frequency.center.to_f64())?
        .add(&Rational::binary(frequency.error.to_f64())?)?;
    // Refine the independent period projection before inversion; a coarse outer bound
    // need not fit inside a tighter production enclosure of the same exact constant.
    for endpoint in oracle::factor(1, 1000, false, 62)? {
        let inverse = Rational::new(1, 1)?.divide(&big(endpoint)?)?;
        assert!(lower.at_most(&inverse)?);
        assert!(inverse.at_most(&upper)?);
    }
    assert!(!Rational::new(636, 1)?.covered(frequency.center.to_f64(), frequency.total.to_f64())?);
    assert!(!Rational::new(-318, 1)?.covered(frequency.center.to_f64(), frequency.total.to_f64())?);
    assert_eq!(
        output.operating_point(&case.heap, &context)?,
        values.map(|value| value[0])
    );
    Ok(())
}

fn big(value: crate::kernel_oracle::Rational) -> TestResult<Rational> {
    Rational::new(
        i64::try_from(value.numerator)?,
        u64::try_from(value.denominator)?,
    )
}

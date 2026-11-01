// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact rational squares distinguish native and composed local LC derivatives.

use crate::reading_testing::TestResult;
use crate::reciprocal_testing::oracle::Rational;
use nrz_scalar::Scalar;

/// A represented point and factor coefficients `(a,b,w)` define `[[a,0],[b,b]] diag(1,w)`.
pub(super) fn cases<P: Scalar>() -> [([P; 2], [P; 3]); 3] {
    let one = P::ONE;
    let hundred = P::from_u8(100);
    let ten = P::from_u8(10);
    [
        (
            [one / P::from_u8(4), one / P::from_u8(16)],
            [one / P::from_u8(2), one / P::from_u8(4), P::from_u8(7)],
        ),
        (
            [one / P::from_u8(40), one / (hundred * hundred * ten)],
            [
                one / (hundred * P::from_u8(40)),
                one / (hundred * hundred * hundred * ten),
                P::from_u8(3),
            ],
        ),
        (
            [one / P::from_u8(8), P::from_u8(3) / P::from_u8(32)],
            [one / P::from_u8(8), one / P::from_u8(16), P::from_u8(7)],
        ),
    ]
}

/// Exact source contraction with a common scalar derivative factor squared.
pub(super) fn targets<P: Scalar>(
    centers: [P; 2],
    covariance: [P; 3],
    intermediate: Option<[P; 2]>,
) -> TestResult<([Rational; 2], Rational)> {
    let [left, right] = centers.map(|value| Rational::binary(value.to_f64()));
    let [left, right] = [left?, right?];
    let product = left.multiply(&right)?;
    let denominator = if let Some([product, root]) = intermediate {
        let product = Rational::binary(product.to_f64())?;
        let root = Rational::binary(root.to_f64())?;
        let square = root.multiply(&root)?;
        Rational::new(4, 1)?
            .multiply(&product)?
            .multiply(&square.multiply(&square)?)?
    } else {
        Rational::new(4, 1)?
            .multiply(&product)?
            .multiply(&product)?
            .multiply(&product)?
    };
    let slopes = [
        right.multiply(&right)?.divide(&denominator)?,
        left.multiply(&left)?.divide(&denominator)?,
    ];
    let cross = product.divide(&denominator)?;
    let [a, b, weight] = covariance.map(|value| Rational::binary(value.to_f64()));
    let [a, b, weight] = [a?, b?, weight?];
    let variance_l = a.multiply(&a)?;
    let covariance_lc = a.multiply(&b)?;
    let variance_c = b
        .multiply(&b)?
        .multiply(&Rational::new(1, 1)?.add(&weight)?)?;
    let variance = slopes[0]
        .multiply(&variance_l)?
        .add(&slopes[1].multiply(&variance_c)?)?
        .add(
            &Rational::new(2, 1)?
                .multiply(&cross)?
                .multiply(&covariance_lc)?,
        )?;
    Ok((slopes, variance))
}

/// Negative derivative signs and exact squared endpoints avoid a numerical square-root oracle.
pub(super) fn negative_square(center: f64, radius: f64, expected: &Rational) -> TestResult {
    let magnitude = Rational::binary(-center)?;
    let low = magnitude.add(&Rational::binary(-radius)?)?;
    let high = magnitude.add(&Rational::binary(radius)?)?;
    assert!(center < 0.0 && radius >= 0.0);
    assert!(Rational::new(0, 1)?.at_most(&low)?);
    assert!(low.multiply(&low)?.at_most(expected)?);
    assert!(expected.at_most(&high.multiply(&high)?)?);
    Ok(())
}

/// Compare rational targets without rounding away the original/intermediate-point distinction.
pub(super) fn equal(left: &Rational, right: &Rational) -> TestResult<bool> {
    Ok(left.at_most(right)? && right.at_most(left)?)
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Correlated quotient oracles retain both signed partials and every shared source.

use super::{TestResult, pair};
use crate::kernel_oracle::Rational;
use crate::statistics_testing::check;
use nrz_bindings::{Budget, Descriptor, Limits, SensitivityRecipe};
use nrz_matrix::MomentAssurance;
use nrz_scalar::Scalar;

/// The cross term has the sign of the two local coefficients.
/// ´test:integration:quantity-quotient-covariance-oracle´
/// ´claim:matrix:quantity-quotient-covariance-oracle´ Correlated quantity quotients enclose the independent rational variance one quarter and signed Jacobian, while repeated-source division encloses zero sensitivity and covariance in both native formats.
#[test]
fn quantity_quotient_covariance_oracle() -> TestResult {
    oracle::<f32>()?;
    oracle::<f64>()
}

/// Expand the covariance independently and then exercise exact shared-source cancellation.
fn oracle<P: Scalar + 'static>() -> TestResult {
    let ratio = Descriptor::ratio();
    let mut case = pair::<P>([&ratio, &ratio])?;
    let result = case.context.divide(
        &mut case.heap,
        &case.values[0],
        &case.values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    let target = case
        .heap
        .companion(result.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = case.heap.moments(case.heap.moment_companion(target)?)?;
    let left = Rational::new(1, 2)?;
    let right = Rational::new(-3, 4)?;
    let variance = left
        .multiply(left)?
        .multiply(Rational::new(1, 4)?)?
        .add(right.multiply(right)?.multiply(Rational::new(1, 2)?)?)?
        .add(left.multiply(right)?.multiply(Rational::new(1, 4)?)?)?;
    assert_eq!((variance.numerator, variance.denominator), (1, 4));
    check(moments.covariance(), &[variance])?;
    check(moments.jacobian().ok_or("Jacobian")?, &[left, right])?;
    assert_eq!(moments.assurance(), MomentAssurance::LinearizedMoments);
    assert!(moments.means().is_none());
    let proof = moments.checked_sensitivity().ok_or("proof")?;
    assert_eq!(proof.recipe(), SensitivityRecipe::QuantityQuotient);
    assert_eq!(
        proof.obligations(),
        &[nrz_bindings::SensitivityObligation::Nonzero { coordinate: 1 }]
    );
    assert_eq!(proof.outputs(), std::slice::from_ref(result.descriptor()));
    let repeated = case.context.divide(
        &mut case.heap,
        &case.values[0],
        &case.values[0],
        &mut Budget::new(Limits::default())?,
    )?;
    let target = case
        .heap
        .companion(repeated.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = case.heap.moments(case.heap.moment_companion(target)?)?;
    check(moments.covariance(), &[Rational::new(0, 1)?])?;
    check(
        moments.sensitivity().ok_or("sensitivity")?,
        &[Rational::new(0, 1)?],
    )?;
    Ok(())
}

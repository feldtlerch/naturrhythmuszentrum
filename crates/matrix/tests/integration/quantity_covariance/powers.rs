// SPDX-License-Identifier: AGPL-3.0-only
//! Exact rational power oracles and cumulative caller receipts.

use super::{TestResult, pair};
use crate::kernel_oracle::Rational;
use crate::statistics_testing::check;
use nrz_bindings::{Budget, Descriptor, Limits, SensitivityRecipe};
use nrz_matrix::MomentAssurance;
use nrz_scalar::Scalar;

/// Signed powers and the formal zero and one cases retain the original source factor.
/// ´test:integration:quantity-power-covariance-oracle´
/// ´claim:matrix:quantity-power-covariance-oracle´ Signed integer powers enclose independently expanded rational derivatives and variances, including zero and one, with linearized assurance and no output mean in both native formats.
#[test]
fn quantity_power_covariance_oracle() -> TestResult {
    oracle::<f32>()?;
    oracle::<f64>()
}

/// The source variance is one quarter and the represented operating coordinate is three.
fn oracle<P: Scalar + 'static>() -> TestResult {
    for (exponent, numerator, denominator) in [
        (-2, -2, 27),
        (-1, -1, 9),
        (0, 0, 1),
        (1, 1, 1),
        (2, 6, 1),
        (3, 27, 1),
    ] {
        let ratio = Descriptor::ratio();
        let mut case = pair::<P>([&ratio, &ratio])?;
        let result = case.context.integer_power(
            &mut case.heap,
            &case.values[0],
            exponent,
            &mut Budget::new(Limits::default())?,
        )?;
        let target = case
            .heap
            .companion(result.view(&case.heap)?.bound().ok_or("bound")?)?
            .target();
        let moments = case.heap.moments(case.heap.moment_companion(target)?)?;
        let derivative = Rational::new(numerator, denominator)?;
        check(moments.jacobian().ok_or("Jacobian")?, &[derivative])?;
        check(
            moments.covariance(),
            &[derivative
                .multiply(derivative)?
                .multiply(Rational::new(1, 4)?)?],
        )?;
        assert_eq!(moments.assurance(), MomentAssurance::LinearizedMoments);
        assert!(moments.means().is_none());
        let proof = moments.checked_sensitivity().ok_or("sensitivity")?;
        assert_eq!(
            proof.recipe(),
            SensitivityRecipe::QuantityIntegerPower { exponent }
        );
        assert_eq!(
            proof.inputs(),
            std::slice::from_ref(case.values[0].descriptor())
        );
        assert_eq!(proof.outputs(), std::slice::from_ref(result.descriptor()));
        assert_eq!(proof.obligations().len(), usize::from(exponent <= 0));
        assert!(proof.jacobian().entries()[0].identity().is_discharged());
    }
    Ok(())
}

/// The integer parameter qualifies reuse even when all ratio descriptors coincide.
/// ´test:integration:quantity-power-covariance-proof-reuse´
/// ´claim:matrix:quantity-power-covariance-proof-reuse´ Equal power descriptors with different exponents retain distinct symbolic identities, while a repeated exponent reuses its identity and evaluates the new operating point and covariance.
#[test]
fn quantity_power_covariance_proof_reuse() -> TestResult {
    let ratio = Descriptor::ratio();
    let mut case = pair::<f64>([&ratio, &ratio])?;
    let square = case.context.integer_power(
        &mut case.heap,
        &case.values[0],
        2,
        &mut Budget::new(Limits::default())?,
    )?;
    let identity = super::proof_identity(&case.heap, &square)?;
    let cube = case.context.integer_power(
        &mut case.heap,
        &case.values[0],
        3,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_ne!(super::proof_identity(&case.heap, &cube)?, identity);
    let repeated = case.context.integer_power(
        &mut case.heap,
        &case.values[1],
        2,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(super::proof_identity(&case.heap, &repeated)?, identity);
    let target = case
        .heap
        .companion(repeated.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = case.heap.moments(case.heap.moment_companion(target)?)?;
    check(
        moments.jacobian().ok_or("Jacobian")?,
        &[Rational::new(4, 1)?],
    )?;
    check(moments.covariance(), &[Rational::new(8, 1)?])?;
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Statistical smooth-domain refusals retain complete deterministic-box authority.

use super::TestResult;
use crate::integration::quantity_products::bounded;
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_matrix::{
    ExecutionLimits, Heap, HeapLimits, Quantity, QuantityContext, QuantityError, ShapeLimits,
};

/// Attach variance only after the original complete physical box is imported.
fn fixture(center: f64, radius: f64) -> TestResult<(QuantityContext, Heap, Quantity<f64>)> {
    let context = QuantityContext::new(
        crate::kernel_testing::owner()?,
        ShapeLimits::default(),
        ExecutionLimits::default(),
    );
    let mut heap = Heap::new(HeapLimits::default())?;
    let input = bounded(
        &context,
        &mut heap,
        &Descriptor::ratio(),
        center,
        radius,
        0.0,
    )?;
    let target = heap
        .companion(input.view(&heap)?.bound().ok_or("bound")?)?
        .target();
    crate::statistics_testing::attach(&mut heap, target, &[0.25], false)?;
    Ok((context, heap, input))
}

/// Nonpositive powers retain their original separated-divisor obligation.
/// ´test:integration:quantity-power-covariance-domain-refusals´
/// ´claim:matrix:quantity-power-covariance-domain-refusals´ Statistical nonpositive powers reject zero-containing complete boxes atomically, retain spent Work and accept fresh boxes of either separated sign.
#[test]
fn quantity_power_covariance_domain_refusals() -> TestResult {
    for exponent in [-1, 0] {
        let (context, mut heap, input) = fixture(1.0, 1.0)?;
        let before = heap.provenance_usage();
        let mut budget = Budget::new(Limits::default())?;
        let result = context.integer_power(&mut heap, &input, exponent, &mut budget);
        let Err(QuantityError::Execution(failure)) = result else {
            panic!("zero-containing original divisor must refuse");
        };
        assert!(matches!(
            failure.cause(),
            nrz_matrix::OperationError::Kernel(nrz_matrix::KernelError::Quantity {
                cause: nrz_scalar::KernelError::Arithmetic(
                    nrz_scalar::ArithmeticFailure::ZeroInDivisor
                ),
                ..
            })
        ));
        assert_eq!(heap.provenance_usage(), before);
        assert!(budget.usage().work > 0);
        for center in [-2.0, 2.0] {
            let (context, mut heap, input) = fixture(center, 0.25)?;
            context.integer_power(
                &mut heap,
                &input,
                exponent,
                &mut Budget::new(Limits::default())?,
            )?;
        }
    }
    Ok(())
}

/// Positive exponents retain their smooth coefficient at a zero operating point.
/// ´test:integration:quantity-power-covariance-zero-point´
/// ´claim:matrix:quantity-power-covariance-zero-point´ Positive integer powers accept zero-centered boxes, retaining derivative one for the unit power and zero for square and cube without an artificial divisor premise.
#[test]
fn quantity_power_covariance_zero_point() -> TestResult {
    for exponent in [1, 2, 3] {
        let (context, mut heap, input) = fixture(0.0, 0.25)?;
        let result = context.integer_power(
            &mut heap,
            &input,
            exponent,
            &mut Budget::new(Limits::default())?,
        )?;
        let target = heap
            .companion(result.view(&heap)?.bound().ok_or("bound")?)?
            .target();
        let moments = heap.moments(heap.moment_companion(target)?)?;
        let derivative = i128::from(exponent == 1);
        crate::statistics_testing::check(
            moments.jacobian().ok_or("Jacobian")?,
            &[crate::kernel_oracle::Rational::new(derivative, 1)?],
        )?;
        crate::statistics_testing::check(
            moments.covariance(),
            &[crate::kernel_oracle::Rational::new(derivative, 4)?],
        )?;
        assert_eq!(
            moments.checked_sensitivity().ok_or("proof")?.obligations(),
            [] as [nrz_bindings::SensitivityObligation; 0],
        );
    }
    Ok(())
}

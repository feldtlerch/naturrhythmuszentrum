// SPDX-License-Identifier: AGPL-3.0-only
//! Noncoherent power coordinates retain exact original source factors.

use super::{TestResult, append_pair};
use crate::kernel_oracle::Rational;
use crate::statistics_testing::check;
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_matrix::{ExecutionLimits, Heap, HeapLimits, Quantity, QuantityContext, ShapeLimits};

/// Build only an exact descriptor scale, without changing the coherent source declaration.
fn scaled(name: &str, scale: i64) -> TestResult<Descriptor> {
    use nrz_bindings::readings::algebra::Rational as Exact;
    Ok(Descriptor::new(
        nrz_bindings::Kind::Ratio,
        nrz_bindings::Measure::new(
            name,
            nrz_bindings::Scale::Exact(Exact::from_ratio(
                scale,
                1,
                &mut Budget::new(Limits::default())?,
            )?),
            Exact::zero(),
        )?,
        nrz_bindings::Role::Difference,
    )?)
}

/// Conversion factors remain part of source composition rather than the local coordinate law.
/// ´test:integration:quantity-power-covariance-measures´
/// ´claim:matrix:quantity-power-covariance-measures´ Noncoherent powers retain exact composed measures and independently checked source factors from scale two, with rational variance `9/4`.
#[test]
fn quantity_power_covariance_measures() -> TestResult {
    let ratio = Descriptor::ratio();
    let first = scaled("twice", 2)?;
    let context = QuantityContext::new(
        crate::kernel_testing::owner()?,
        ShapeLimits::default(),
        ExecutionLimits::default(),
    );
    let mut case = append_pair(
        context,
        Heap::new(HeapLimits::default())?,
        [&ratio, &ratio],
        [6.0_f64, 16.0],
    )?;
    let left = case.context.convert(
        &mut case.heap,
        &case.values[0],
        &first,
        &mut Budget::new(Limits::default())?,
    )?;
    let square = case.context.integer_power(
        &mut case.heap,
        &left,
        2,
        &mut Budget::new(Limits::default())?,
    )?;
    verify(&case.heap, &square, &[(6, 1)], &[(3, 1)], (9, 4))?;
    let registry = case.context.declarations().registry();
    assert_eq!(
        square.descriptor(),
        &registry.coordinate_integer_power(&first, 2, &mut Budget::new(Limits::default())?)?
    );
    Ok(())
}

/// Compare local and composed derivatives with independent rational constants.
fn verify(
    heap: &Heap,
    value: &Quantity<f64>,
    jacobian: &[(i128, i128)],
    sensitivity: &[(i128, i128)],
    variance: (i128, i128),
) -> TestResult {
    let target = heap
        .companion(value.view(heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = heap.moments(heap.moment_companion(target)?)?;
    let jacobian = jacobian
        .iter()
        .map(|&(n, d)| Rational::new(n, d))
        .collect::<TestResult<Vec<_>>>()?;
    let sensitivity = sensitivity
        .iter()
        .map(|&(n, d)| Rational::new(n, d))
        .collect::<TestResult<Vec<_>>>()?;
    check(moments.jacobian().ok_or("Jacobian")?, &jacobian)?;
    check(moments.sensitivity().ok_or("sensitivity")?, &sensitivity)?;
    check(
        moments.covariance(),
        &[Rational::new(variance.0, variance.1)?],
    )?;
    let proof = moments.checked_sensitivity().ok_or("proof")?;
    assert_eq!(proof.outputs(), std::slice::from_ref(value.descriptor()));
    Ok(())
}

/// Conversion factors remain part of source composition rather than the local coordinate law.
/// ´test:integration:quantity-nonlinear-covariance-measures´
/// ´claim:matrix:quantity-nonlinear-covariance-measures´ Noncoherent powers, roots and quotients retain exact composed measures and independently checked source factors from scales two and four, with rational variances `9/4`, `1/512` and `29/8192`.
#[test]
fn quantity_nonlinear_covariance_measures() -> TestResult {
    let ratio = Descriptor::ratio();
    let first = scaled("twice", 2)?;
    let second = scaled("fourfold", 4)?;
    let context = QuantityContext::new(
        crate::kernel_testing::owner()?,
        ShapeLimits::default(),
        ExecutionLimits::default(),
    );
    let mut case = append_pair(
        context,
        Heap::new(HeapLimits::default())?,
        [&ratio, &ratio],
        [6.0_f64, 16.0],
    )?;
    let left = case.context.convert(
        &mut case.heap,
        &case.values[0],
        &first,
        &mut Budget::new(Limits::default())?,
    )?;
    let right = case.context.convert(
        &mut case.heap,
        &case.values[1],
        &second,
        &mut Budget::new(Limits::default())?,
    )?;
    let square = case.context.integer_power(
        &mut case.heap,
        &left,
        2,
        &mut Budget::new(Limits::default())?,
    )?;
    verify(&case.heap, &square, &[(6, 1)], &[(3, 1)], (9, 4))?;
    let root =
        case.context
            .square_root(&mut case.heap, &right, &mut Budget::new(Limits::default())?)?;
    verify(&case.heap, &root, &[(1, 4)], &[(1, 16)], (1, 512))?;
    let quotient = case.context.divide(
        &mut case.heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    let a = Rational::new(1, 8)?;
    let b = Rational::new(-3, 64)?;
    let variance = a
        .multiply(a)?
        .multiply(Rational::new(1, 4)?)?
        .add(b.multiply(b)?.multiply(Rational::new(1, 2)?)?)?
        .add(a.multiply(b)?.multiply(Rational::new(1, 4)?)?)?;
    verify(
        &case.heap,
        &quotient,
        &[(1, 4), (-3, 16)],
        &[(1, 8), (-3, 64)],
        (variance.numerator, variance.denominator),
    )?;
    assert_eq!((variance.numerator, variance.denominator), (29, 8192));
    let registry = case.context.declarations().registry();
    assert_eq!(
        square.descriptor(),
        &registry.coordinate_integer_power(&first, 2, &mut Budget::new(Limits::default())?)?
    );
    assert_eq!(
        root.descriptor(),
        &registry.coordinate_square_root(&second, &mut Budget::new(Limits::default())?)?
    );
    assert_eq!(
        quotient.descriptor(),
        &registry.coordinate_binary(
            nrz_bindings::BinaryOp::Divide,
            &first,
            &second,
            &mut Budget::new(Limits::default())?
        )?
    );
    Ok(())
}

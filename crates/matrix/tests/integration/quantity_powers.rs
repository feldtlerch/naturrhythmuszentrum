// SPDX-License-Identifier: AGPL-3.0-only
//! Exact unary quantity powers retain measures, scalar certificates and original domains.

mod boundaries;
mod lineage;
mod refusals;

use super::quantity_products::{bounded, channels, check_result, descriptor, evidence};
use crate::conversion_testing::TestResult;
use crate::kernel_oracle::Rational;
use nrz_bindings::readings::algebra::Rational as Exact;
use nrz_bindings::{
    Budget, Context, Descriptor, Dimension, ExponentPolicy, Kind, Limits, Measure, Registry, Role,
};
use nrz_matrix::{
    ExecutionLimits, Heap, HeapLimits, Quantity, QuantityContext, QuantityError, ShapeLimits,
};
use nrz_scalar::Scalar;
use std::rc::Rc;

/// Registered length, area and fractional-length kinds never arise from dimension equality alone.
fn fixture(policy: ExponentPolicy) -> TestResult<(QuantityContext, [Descriptor; 3])> {
    let length = descriptor("length", [1, 0, 0, 0, 0, 0, 0], (1, 1))?;
    let area = descriptor("area", [2, 0, 0, 0, 0, 0, 0], (1, 1))?;
    let mut budget = Budget::new(Limits::default())?;
    let half = Exact::from_ratio(2, 4, &mut budget)?;
    let mut powers = std::array::from_fn(|_| Exact::zero());
    powers[0] = half.clone();
    let root_length = Descriptor::new(
        Kind::Named("root-length".into(), Box::new(Dimension::new(powers)?)),
        Measure::coherent(),
        Role::Difference,
    )?;
    let mut registry = Registry::new(policy);
    registry.register_power(
        length.kind().clone(),
        Exact::from_ratio(2, 1, &mut budget)?,
        area.kind().clone(),
    )?;
    registry.register_power(length.kind().clone(), Exact::zero(), Kind::Ratio)?;
    registry.register_power(area.kind().clone(), half.clone(), length.kind().clone())?;
    if policy == ExponentPolicy::Rational {
        registry.register_power(length.kind().clone(), half, root_length.kind().clone())?;
    }
    let declarations = Context::with_real_symbols(registry, Limits::default(), &[])?.freeze()?;
    Ok((
        QuantityContext::new(
            Rc::new(declarations),
            ShapeLimits::default(),
            ExecutionLimits::default(),
        ),
        [length, area, root_length],
    ))
}

/// Closed test choice also drives the literal Work and domain boundary fixtures.
#[derive(Clone, Copy)]
enum Unary {
    Power(i64),
    Root,
}

/// Keep every tested law on the public quantity facade and the same caller ledger.
fn execute(
    context: &QuantityContext,
    heap: &mut Heap,
    input: &Quantity<f64>,
    operation: Unary,
    budget: &mut Budget,
) -> Result<Quantity<f64>, QuantityError<f64>> {
    match operation {
        Unary::Power(exponent) => context.integer_power(heap, input, exponent, budget),
        Unary::Root => context.square_root(heap, input, budget),
    }
}

/// Independent physical integer truth is enclosed in both scalar formats.
/// ´test:integration:quantity-powers-exact-oracles´
/// ´claim:matrix:quantity-powers-exact-oracles´ Squaring three metres and rooting nine square metres retain exact measures, the original source and exported certificates in both native formats.
#[test]
fn quantity_powers_exact_oracles() -> TestResult {
    physical::<f32>()?;
    physical::<f64>()
}

/// Run the stated dimensional square/root pair and inspect the original exact exponent.
fn physical<P: Scalar + 'static>() -> TestResult {
    let (context, [length, area, _]) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let input = bounded(
        &context,
        &mut heap,
        &length,
        P::from_u8(3),
        P::ZERO,
        P::ZERO,
    )?;
    let square =
        context.integer_power(&mut heap, &input, 2, &mut Budget::new(Limits::default())?)?;
    check_result(&heap, &square, &area, 9)?;
    let record = square.operation_record(&heap)?;
    assert_eq!(record.operation().quantity_integer_exponent(), Some(2));
    assert_eq!(record.inputs().len(), 1);
    assert_eq!(record.inputs()[0].stamp(), input.view(&heap)?.stamp());
    assert_eq!(
        record.operation().inputs()[0].descriptor(),
        input.descriptor()
    );
    assert_eq!(evidence(&heap, &square)?.certificates().len(), 7);
    let root = context.square_root(&mut heap, &square, &mut Budget::new(Limits::default())?)?;
    check_result(&heap, &root, &length, 3)?;
    assert_eq!(
        root.operation_record(&heap)?
            .operation()
            .quantity_integer_exponent(),
        None
    );
    assert_eq!(evidence(&heap, &root)?.certificates().len(), 7);
    let two = bounded(&context, &mut heap, &area, P::from_u8(2), P::ZERO, P::ZERO)?;
    let irrational = context.square_root(&mut heap, &two, &mut Budget::new(Limits::default())?)?;
    let [center, total, error] = channels(&heap, &irrational)?;
    assert!(error > 0.0);
    assert!(total >= error);
    let center = Rational::binary(center)?;
    for radius in [total, error] {
        let radius = Rational::binary(radius)?;
        // Round the radius inward to a dyadic grid whose squared endpoints fit i128.
        let grid = Rational::new(1, 1_i128 << 60)?;
        let steps = radius.divide(grid)?;
        let inward = Rational::new(steps.numerator / steps.denominator, 1_i128 << 60)?;
        assert!(inward.numerator > 0);
        assert!(inward.at_most(radius)?);
        let lower = center.subtract(inward)?;
        let upper = center.add(inward)?;
        let two = Rational::new(2, 1)?;
        assert!(Rational::new(0, 1)?.at_most(lower)?);
        assert!(lower.multiply(lower)?.at_most(two)?);
        assert!(two.at_most(upper.multiply(upper)?)?);
    }
    Ok(())
}

/// Negative and zero exponents keep strict original domains and exact rational targets.
/// ´test:integration:quantity-powers-signed-oracles´
/// ´claim:matrix:quantity-powers-signed-oracles´ Inversion-first integer powers enclose independent rational truths, including exact one at zero exponent after the original nonzero domain is certified.
#[test]
fn quantity_powers_signed_oracles() -> TestResult {
    let (context, _) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let input = bounded(&context, &mut heap, &Descriptor::ratio(), 3.0_f64, 0.0, 0.0)?;
    for (exponent, numerator, denominator) in [
        (-2, 1, 9),
        (-1, 1, 3),
        (0, 1, 1),
        (1, 3, 1),
        (2, 9, 1),
        (3, 27, 1),
        (4, 81, 1),
    ] {
        let result = context.integer_power(
            &mut heap,
            &input,
            exponent,
            &mut Budget::new(Limits::default())?,
        )?;
        let [center, total, error] = channels(&heap, &result)?;
        let exact = Rational::new(numerator, denominator)?;
        assert!(covered(exact, center, total)?);
        assert!(covered(exact, center, error)?);
        assert_eq!(
            result
                .operation_record(&heap)?
                .operation()
                .quantity_integer_exponent(),
            Some(exponent)
        );
    }
    Ok(())
}

/// Rational exponent reduction and policy admission remain exact registry facts.
/// ´test:integration:quantity-root-exponent-policy´
/// ´claim:matrix:quantity-root-exponent-policy´ An integral-only registry refuses a fractional result dimension while area roots and reduced rational two-fourths registrations are accepted.
#[test]
fn quantity_root_exponent_policy() -> TestResult {
    for policy in [ExponentPolicy::Integral, ExponentPolicy::Rational] {
        let (context, [length, area, root_length]) = fixture(policy)?;
        let mut heap = Heap::new(HeapLimits::default())?;
        let input = bounded(&context, &mut heap, &length, 9.0_f64, 0.0, 0.0)?;
        let result = context.square_root(&mut heap, &input, &mut Budget::new(Limits::default())?);
        if policy == ExponentPolicy::Integral {
            let Err(fault) = result else {
                panic!("fractional root dimension must refuse under integral policy");
            };
            assert!(matches!(
                fault,
                QuantityError::Units(nrz_bindings::UnitError::NonIntegralDimension)
            ));
        } else {
            check_result(&heap, &result?, &root_length, 3)?;
        }
        let area = bounded(&context, &mut heap, &area, 9.0_f64, 0.0, 0.0)?;
        let result = context.square_root(&mut heap, &area, &mut Budget::new(Limits::default())?)?;
        check_result(&heap, &result, &length, 3)?;
    }
    Ok(())
}

/// Exact equality needs only a nonnegative radius, including subnormal radius certificates.
fn covered(exact: Rational, center: f64, radius: f64) -> TestResult<bool> {
    if exact.subtract(Rational::binary(center)?)?.numerator == 0 {
        return Ok(radius >= 0.0);
    }
    exact.covered(center, radius)
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Unary semantic and scalar-domain refusals have accepted neighboring contracts.

use super::{TestResult, Unary, bounded, execute, fixture};
use nrz_bindings::{Budget, Descriptor, ExponentPolicy, Limits, Role, UnitError};
use nrz_matrix::{Heap, HeapLimits, QuantityError};
use nrz_scalar::{ArithmeticFailure, KernelError};

/// Points and attached statistics refuse before any unary output publication.
/// ´test:integration:quantity-powers-semantic-refusals´
/// ´claim:matrix:quantity-powers-semantic-refusals´ Integer powers and roots reject point roles and nonlinear statistics while deterministic difference twins succeed.
#[test]
fn quantity_powers_semantic_refusals() -> TestResult {
    let (context, [length, _, _]) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let statistical = crate::statistics_testing::scalar::<f64>(&mut heap)?;
    let statistical = context.capture(
        &heap,
        heap.symbol_binding(statistical.value())?,
        &mut Budget::new(Limits::default())?,
    )?;
    let point = Descriptor::new(length.kind().clone(), length.measure().clone(), Role::Point)?;
    let point = bounded(&context, &mut heap, &point, 3.0_f64, 0.0, 0.0)?;
    let plain = bounded(&context, &mut heap, &length, 3.0_f64, 0.0, 0.0)?;
    for operation in [Unary::Power(2), Unary::Power(0), Unary::Root] {
        let before = heap.provenance_usage();
        let Err(fault) = execute(
            &context,
            &mut heap,
            &point,
            operation,
            &mut Budget::new(Limits::default())?,
        ) else {
            panic!("unary point arithmetic must refuse");
        };
        assert!(matches!(fault, QuantityError::Units(UnitError::AffineRole)));
        let Err(fault) = execute(
            &context,
            &mut heap,
            &statistical,
            operation,
            &mut Budget::new(Limits::default())?,
        ) else {
            panic!("unary statistical propagation must refuse");
        };
        assert!(matches!(fault, QuantityError::NonlinearStatistics));
        assert_eq!(heap.provenance_usage(), before);
        execute(
            &context,
            &mut heap,
            &plain,
            operation,
            &mut Budget::new(Limits::default())?,
        )?;
    }
    Ok(())
}

/// Zero, negative and zero-touching roots refuse without clamping; reciprocal powers retain domains.
/// ´test:integration:quantity-powers-domain-refusals´
/// ´claim:matrix:quantity-powers-domain-refusals´ Positive-root certificates reject nonpositive boxes, and nonpositive integer powers reject zero-containing original boxes while separated-sign twins succeed.
#[test]
fn quantity_powers_domain_refusals() -> TestResult {
    let (context, _) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let ratio = Descriptor::ratio();
    for (center, total) in [(-1.0_f64, 0.0), (0.0, 0.0), (1.0, 1.0)] {
        let input = bounded(&context, &mut heap, &ratio, center, total, 0.0)?;
        let before = heap.provenance_usage();
        let Err(QuantityError::Execution(failure)) =
            context.square_root(&mut heap, &input, &mut Budget::new(Limits::default())?)
        else {
            panic!("nonpositive root box must refuse");
        };
        assert!(matches!(
            failure.cause(),
            nrz_matrix::OperationError::Kernel(nrz_matrix::KernelError::Quantity {
                cause: KernelError::PositiveRoot,
                ..
            })
        ));
        assert_eq!(heap.provenance_usage(), before);
    }
    let zero = bounded(&context, &mut heap, &ratio, 0.0_f64, 0.0, 0.0)?;
    for exponent in [-1, 0] {
        let before = heap.provenance_usage();
        let Err(QuantityError::Execution(failure)) = context.integer_power(
            &mut heap,
            &zero,
            exponent,
            &mut Budget::new(Limits::default())?,
        ) else {
            panic!("undefined nonpositive power at zero must refuse");
        };
        assert!(matches!(
            failure.cause(),
            nrz_matrix::OperationError::Kernel(nrz_matrix::KernelError::Quantity {
                cause: KernelError::Arithmetic(ArithmeticFailure::ZeroInDivisor),
                ..
            })
        ));
        assert_eq!(heap.provenance_usage(), before);
        for center in [-2.0_f64, 2.0] {
            let input = bounded(&context, &mut heap, &ratio, center, 0.25, 0.125)?;
            context.integer_power(
                &mut heap,
                &input,
                exponent,
                &mut Budget::new(Limits::default())?,
            )?;
        }
    }
    let positive = bounded(&context, &mut heap, &ratio, 2.0_f64, 0.25, 0.125)?;
    context.square_root(&mut heap, &positive, &mut Budget::new(Limits::default())?)?;
    Ok(())
}

/// Arrays do not silently select one entry for the scalar-only unary schedule.
/// ´test:integration:quantity-powers-extent-refusals´
/// ´claim:matrix:quantity-powers-extent-refusals´ Nonscalar powers and roots refuse without publication while scalar quantity fixtures remain supported.
#[test]
fn quantity_powers_extent_refusals() -> TestResult {
    let (context, [length, _, _]) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let mut budget = Budget::new(Limits::default())?;
    let symbols =
        nrz_bindings::BindingSymbols::declare(&length, [2, 1], [true, true], &mut budget)?;
    let input = context.import(
        &mut heap,
        &symbols,
        nrz_matrix::QuantityInput::bounded(&[3.0_f64, 4.0], &[0.0, 0.0], Some(&[0.0, 0.0])),
        &mut budget,
    )?;
    for operation in [Unary::Power(2), Unary::Root] {
        let before = heap.provenance_usage();
        let Err(fault) = execute(
            &context,
            &mut heap,
            &input,
            operation,
            &mut Budget::new(Limits::default())?,
        ) else {
            panic!("nonscalar unary operation must refuse");
        };
        assert!(matches!(fault, QuantityError::NonlinearExtent));
        assert_eq!(heap.provenance_usage(), before);
    }
    let scalar = bounded(&context, &mut heap, &length, 3.0_f64, 0.0, 0.0)?;
    context.integer_power(&mut heap, &scalar, 2, &mut Budget::new(Limits::default())?)?;
    Ok(())
}

/// Exact exponent ceilings refuse before any numerical schedule or output registration.
/// ´test:integration:quantity-power-exponent-ceiling´
/// ´claim:matrix:quantity-power-exponent-ceiling´ An exponent beyond the existing exact ceiling refuses with its named budget while an admitted integer power succeeds without changing that ceiling.
#[test]
fn quantity_power_exponent_ceiling() -> TestResult {
    let (context, _) = fixture(ExponentPolicy::Rational)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let input = bounded(&context, &mut heap, &Descriptor::ratio(), 3.0_f64, 0.0, 0.0)?;
    let before = heap.provenance_usage();
    let mut budget = Budget::new(Limits::default())?;
    let Err(QuantityError::Units(UnitError::Arithmetic(fault))) =
        context.integer_power(&mut heap, &input, 257, &mut budget)
    else {
        panic!("exact integer exponent ceiling must refuse");
    };
    assert!(matches!(
        fault.kind,
        nrz_bindings::readings::algebra::FaultKind::Exhausted {
            budget: nrz_bindings::readings::algebra::limits::BudgetKind::IntegerExponent,
            limit: 256,
            ..
        }
    ));
    assert!(budget.usage().work > 0);
    assert_eq!(heap.provenance_usage(), before);
    context.integer_power(&mut heap, &input, 2, &mut Budget::new(Limits::default())?)?;
    Ok(())
}

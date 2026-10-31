// SPDX-License-Identifier: AGPL-3.0-only
//! Every reached nonlinear refusal has an accepted neighboring contract.

use super::{TestResult, bounded, channels, fixture};
use nrz_bindings::{Budget, Descriptor, Limits, Role, UnitError};
use nrz_matrix::{Heap, HeapLimits, QuantityError};
use nrz_scalar::{ArithmeticFailure, KernelError};

/// Point roles and statistical companions refuse without numerical or heap publication.
/// ´test:integration:quantity-products-semantic-refusals´
/// ´claim:matrix:quantity-products-semantic-refusals´ Point multiplication and statistical nonlinear transport refuse while difference and deterministic twins succeed.
#[test]
fn quantity_products_semantic_refusals() -> TestResult {
    let (context, [voltage, current, _]) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let statistical = crate::statistics_testing::scalar::<f64>(&mut heap)?;
    let point = Descriptor::new(
        voltage.kind().clone(),
        voltage.measure().clone(),
        Role::Point,
    )?;
    let left = bounded(&context, &mut heap, &point, 2.0_f64, 0.0, 0.0)?;
    let right = bounded(&context, &mut heap, &current, 3.0, 0.0, 0.0)?;
    let Err(fault) = context.multiply(
        &mut heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    ) else {
        panic!("point product must refuse");
    };
    assert!(matches!(fault, QuantityError::Units(UnitError::AffineRole)));
    let difference = bounded(&context, &mut heap, &voltage, 2.0_f64, 0.0, 0.0)?;
    context.multiply(
        &mut heap,
        &difference,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    let statistical = context.capture(
        &heap,
        heap.symbol_binding(statistical.value())?,
        &mut Budget::new(Limits::default())?,
    )?;
    let Err(fault) = context.multiply(
        &mut heap,
        &statistical,
        &difference,
        &mut Budget::new(Limits::default())?,
    ) else {
        panic!("statistical product must refuse");
    };
    assert!(matches!(fault, QuantityError::NonlinearStatistics));
    let standard = Descriptor::new(
        nrz_bindings::Kind::Uncertainty(
            nrz_bindings::UncertaintyMode::Standard,
            Box::new(nrz_bindings::Kind::Ratio),
            None,
        ),
        nrz_bindings::Measure::coherent(),
        Role::Difference,
    )?;
    let standard = bounded(&context, &mut heap, &standard, 2.0_f64, 0.0, 0.0)?;
    let Err(fault) = context.multiply(
        &mut heap,
        &standard,
        &difference,
        &mut Budget::new(Limits::default())?,
    ) else {
        panic!("standard uncertainty quantity must refuse");
    };
    assert!(matches!(fault, QuantityError::NonlinearStatistics));
    let plain = bounded(&context, &mut heap, &Descriptor::ratio(), 2.0_f64, 0.0, 0.0)?;
    context.multiply(
        &mut heap,
        &plain,
        &difference,
        &mut Budget::new(Limits::default())?,
    )?;
    Ok(())
}

/// A zero-straddling total refuses even when the represented divisor is nonzero.
/// ´test:integration:quantity-quotient-domain-refusal´
/// ´claim:matrix:quantity-quotient-domain-refusal´ Division requires nonzero total coverage, preserves spent Work on refusal and accepts either separated sign.
#[test]
fn quantity_quotient_domain_refusal() -> TestResult {
    let (context, _) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let left = bounded(&context, &mut heap, &Descriptor::ratio(), 3.0_f64, 0.0, 0.0)?;
    let right = bounded(
        &context,
        &mut heap,
        &Descriptor::ratio(),
        1.0_f64,
        2.0,
        0.125,
    )?;
    let mut budget = Budget::new(Limits::default())?;
    let Err(fault) = context.divide(&mut heap, &left, &right, &mut budget) else {
        panic!("zero-containing divisor must refuse");
    };
    let QuantityError::Execution(failure) = fault else {
        panic!("scalar domain execution refusal");
    };
    assert!(matches!(
        failure.cause(),
        nrz_matrix::OperationError::Kernel(nrz_matrix::KernelError::Quantity {
            cause: KernelError::Arithmetic(ArithmeticFailure::ZeroInDivisor),
            ..
        })
    ));
    assert!(budget.usage().work > 0);
    for center in [-2.0_f64, 2.0] {
        let right = bounded(
            &context,
            &mut heap,
            &Descriptor::ratio(),
            center,
            0.25,
            0.125,
        )?;
        let output = context.divide(
            &mut heap,
            &left,
            &right,
            &mut Budget::new(Limits::default())?,
        )?;
        assert_eq!(channels(&heap, &output)?[0], 3.0 / center);
    }
    Ok(())
}

/// Nonlinear sources retain their original versions through coordinate conversion.
/// ´test:integration:quantity-products-live-evidence´
/// ´claim:matrix:quantity-products-live-evidence´ Nonlinear result access and converted descendants refuse a changed original source version while unchanged sources remain valid.
#[test]
fn quantity_products_live_evidence() -> TestResult {
    let (context, _) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let ratio = Descriptor::ratio();
    let left = bounded(&context, &mut heap, &ratio, 3.0_f64, 0.25, 0.125)?;
    let right = bounded(&context, &mut heap, &ratio, 2.0_f64, 0.25, 0.125)?;
    let result = context.multiply(
        &mut heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    let converted = context.convert(
        &mut heap,
        &result,
        &ratio,
        &mut Budget::new(Limits::default())?,
    )?;
    let narrowed = context.narrow(&mut heap, &result, &mut Budget::new(Limits::default())?)?;
    result.operation_record(&heap)?;
    converted.operation_record(&heap)?;
    channels(&heap, &converted)?;
    channels(&heap, &narrowed)?;
    let view = left.view(&heap)?;
    let stamp = view.stamp();
    let handle = heap
        .companion(view.bound().ok_or("original bound")?)?
        .target();
    heap.replace_represented(handle, stamp.version(), &[3.0])?;
    for quantity in [&result, &converted] {
        let Err(fault) = quantity.view(&heap) else {
            panic!("changed nonlinear source must refuse");
        };
        assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
    }
    let Err(fault) = narrowed.view(&heap) else {
        panic!("changed source must invalidate a precision-converted descendant");
    };
    assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
    assert_eq!(right.view(&heap)?.region().get(0, 0)?, 2.0);
    Ok(())
}

/// Raw heap access and binding recapture retain the original nonlinear source dependencies.
/// ´test:integration:quantity-products-heap-provenance´
/// ´claim:matrix:quantity-products-heap-provenance´ Original source replacement invalidates quantity, raw heap and recaptured binding access without a separate history authority.
#[test]
fn quantity_products_heap_provenance() -> TestResult {
    let (context, _) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let ratio = Descriptor::ratio();
    let left = bounded(&context, &mut heap, &ratio, 3.0_f64, 0.25, 0.125)?;
    let right = bounded(&context, &mut heap, &ratio, 2.0_f64, 0.25, 0.125)?;
    let result = context.multiply(
        &mut heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    let recaptured = context.capture(
        &heap,
        result.binding().clone(),
        &mut Budget::new(Limits::default())?,
    )?;
    let view = left.view(&heap)?;
    let version = view.stamp().version();
    let handle = heap.companion(view.bound().ok_or("left bound")?)?.target();
    heap.replace_represented(handle, version, &[3.0])?;
    let Err(fault) = result.view(&heap) else {
        panic!("quantity-owned evidence must detect the changed source");
    };
    assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
    let Err(fault) = heap.value_by_symbol(result.symbol(), result.binding()) else {
        panic!("stale nonlinear heap lookup must refuse");
    };
    assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
    let Err(fault) = recaptured.view(&heap) else {
        panic!("recaptured stale nonlinear binding must refuse");
    };
    assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
    Ok(())
}

/// Mixed deterministic scopes refuse, while two represented operands retain represented coverage.
/// ´test:integration:quantity-products-coverage-refusal´
/// ´claim:matrix:quantity-products-coverage-refusal´ A product never upgrades represented-input coverage to declared physical bounds and an equal-scope twin succeeds.
#[test]
fn quantity_products_coverage_refusal() -> TestResult {
    let (context, _) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let ratio = Descriptor::ratio();
    let represented =
        heap.import_represented(nrz_matrix::Extent::new(1, 1)?, &ratio, &[2.0_f64])?;
    let represented = context.capture(
        &heap,
        heap.symbol_binding(represented.value())?,
        &mut Budget::new(Limits::default())?,
    )?;
    let declared = bounded(&context, &mut heap, &ratio, 3.0, 0.0, 0.0)?;
    let before = heap.provenance_usage();
    let Err(fault) = context.multiply(
        &mut heap,
        &represented,
        &declared,
        &mut Budget::new(Limits::default())?,
    ) else {
        panic!("mixed deterministic coverage must refuse");
    };
    let QuantityError::Operation(fault) = fault else {
        panic!("coverage must refuse during binding");
    };
    assert!(matches!(
        *fault,
        nrz_matrix::OperationError::Kernel(nrz_matrix::KernelError::CoverageMismatch)
    ));
    assert_eq!(heap.provenance_usage(), before);
    let result = context.multiply(
        &mut heap,
        &represented,
        &represented,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(
        result.view(&heap)?.coverage(),
        nrz_matrix::Coverage::RepresentedInputs
    );
    Ok(())
}

/// Semantic and extent admission reject unsupported operations before scalar evaluation.
/// ´test:integration:quantity-products-shape-kind-refusals´
/// ´claim:matrix:quantity-products-shape-kind-refusals´ Unregistered physical products and nonscalar operands refuse while registered scalar twins publish.
#[test]
fn quantity_products_shape_kind_refusals() -> TestResult {
    let (context, [voltage, current, _]) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let left = bounded(&context, &mut heap, &voltage, 2.0_f64, 0.0, 0.0)?;
    let Err(fault) = context.multiply(
        &mut heap,
        &left,
        &left,
        &mut Budget::new(Limits::default())?,
    ) else {
        panic!("unregistered voltage-squared kind must refuse");
    };
    assert!(matches!(
        fault,
        QuantityError::Units(UnitError::UnsupportedKindRule)
    ));
    let mut budget = Budget::new(Limits::default())?;
    let symbols =
        nrz_bindings::BindingSymbols::declare(&current, [2, 1], [true, true], &mut budget)?;
    let array = context.import(
        &mut heap,
        &symbols,
        nrz_matrix::QuantityInput::bounded(&[3.0_f64, 4.0], &[0.0, 0.0], Some(&[0.0, 0.0])),
        &mut budget,
    )?;
    let before = heap.provenance_usage();
    let Err(fault) = context.multiply(
        &mut heap,
        &left,
        &array,
        &mut Budget::new(Limits::default())?,
    ) else {
        panic!("nonscalar quantity product must refuse");
    };
    assert!(matches!(fault, QuantityError::NonlinearExtent));
    assert_eq!(heap.provenance_usage(), before);
    let right = bounded(&context, &mut heap, &current, 3.0_f64, 0.0, 0.0)?;
    let output = context.multiply(
        &mut heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(channels(&heap, &output)?[0], 6.0);
    Ok(())
}

/// Rebinding a quantity recipe cannot bypass the named statistics refusal.
/// ´test:integration:quantity-products-statistical-rebinding´
/// ´claim:matrix:quantity-products-statistical-rebinding´ Direct binding of a quantity recipe to a statistical operand returns `NonlinearStatistics` before evaluation and preserves heap provenance.
#[test]
fn quantity_products_statistical_rebinding() -> TestResult {
    let (context, _) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let statistical = crate::statistics_testing::scalar::<f64>(&mut heap)?;
    let represented = heap.import_represented(
        nrz_matrix::Extent::new(1, 1)?,
        &Descriptor::ratio(),
        &[2.0_f64],
    )?;
    let plain = context.capture(
        &heap,
        heap.symbol_binding(represented.value())?,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(
        plain.view(&heap)?.coverage(),
        heap.value(statistical.value())?.coverage()
    );
    let result = context.multiply(
        &mut heap,
        &plain,
        &plain,
        &mut Budget::new(Limits::default())?,
    )?;
    let operation = result.operation_record(&heap)?.operation().clone();
    assert!(!operation.carries_statistics());
    let before = heap.provenance_usage();
    let Err(fault) = nrz_matrix::Binding::<f64, f64>::from_symbols(
        operation,
        vec![
            heap.symbol_binding(statistical.value())?,
            plain.binding().clone(),
        ],
        nrz_matrix::ExecutionLimits::default(),
    )
    .validate(&mut heap) else {
        panic!("statistical quantity recipe rebinding must refuse");
    };
    assert!(
        matches!(
            fault,
            nrz_matrix::OperationError::Statistics(
                nrz_matrix::StatisticsError::NonlinearStatistics
            )
        ),
        "{fault:?}"
    );
    assert_eq!(heap.provenance_usage(), before);
    Ok(())
}

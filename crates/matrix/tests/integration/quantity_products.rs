// SPDX-License-Identifier: AGPL-3.0-only
//! Scalar quantity products retain exact coordinate measures and complete enclosure evidence.

mod boundaries;
mod refusals;

use crate::conversion_testing::TestResult;
use crate::kernel_oracle::Rational;
use nrz_bindings::readings::algebra::Rational as Exact;
use nrz_bindings::{
    BinaryOp, BindingSymbols, Budget, Context, Descriptor, Dimension, ExponentPolicy, Kind, Limits,
    Measure, Registry, Role, Scale,
};
use nrz_matrix::{
    ExecutionLimits, Heap, HeapLimits, Quantity, QuantityContext, QuantityInput, ShapeLimits,
};
use nrz_scalar::{KernelLaw, Scalar};
use std::rc::Rc;

/// Exact physical fixture with independently stated SI exponents and scale.
fn descriptor(name: &str, exponents: [i64; 7], scale: (i64, u64)) -> TestResult<Descriptor> {
    let mut budget = Budget::new(Limits::default())?;
    Ok(Descriptor::new(
        Kind::Named(name.into(), Box::new(Dimension::integers(exponents)?)),
        Measure::new(
            name,
            Scale::Exact(Exact::from_ratio(scale.0, scale.1, &mut budget)?),
            Exact::zero(),
        )?,
        Role::Difference,
    )?)
}

/// Declare voltage-current-power relations without deriving kinds from dimensions.
fn fixture() -> TestResult<(QuantityContext, [Descriptor; 3])> {
    let voltage = descriptor("voltage", [2, 1, -3, -1, 0, 0, 0], (1, 1))?;
    let current = descriptor("current", [0, 0, 0, 1, 0, 0, 0], (1, 1))?;
    let power = descriptor("power", [2, 1, -3, 0, 0, 0, 0], (1, 1))?;
    let mut registry = Registry::new(ExponentPolicy::Rational);
    registry.register_binary(
        BinaryOp::Multiply,
        voltage.kind().clone(),
        current.kind().clone(),
        power.kind().clone(),
    )?;
    registry.register_binary(
        BinaryOp::Divide,
        power.kind().clone(),
        current.kind().clone(),
        voltage.kind().clone(),
    )?;
    let declarations = Context::with_real_symbols(registry, Limits::default(), &[])?.freeze()?;
    Ok((
        QuantityContext::new(
            Rc::new(declarations),
            ShapeLimits::default(),
            ExecutionLimits::default(),
        ),
        [voltage, current, power],
    ))
}

/// Import explicit complete deterministic and numerical-only scalar channels.
fn bounded<P: Scalar + 'static>(
    context: &QuantityContext,
    heap: &mut Heap,
    descriptor: &Descriptor,
    center: P,
    total: P,
    error: P,
) -> TestResult<Quantity<P>> {
    let mut budget = Budget::new(Limits::default())?;
    let symbols = BindingSymbols::declare(descriptor, [1, 1], [true, true], &mut budget)?;
    Ok(context.import(
        heap,
        &symbols,
        QuantityInput::bounded(
            std::slice::from_ref(&center),
            std::slice::from_ref(&total),
            Some(std::slice::from_ref(&error)),
        ),
        &mut budget,
    )?)
}

/// Every independently stored companion targets the fresh value version.
fn channels<P: Scalar + 'static>(heap: &Heap, quantity: &Quantity<P>) -> TestResult<[f64; 3]> {
    let view = quantity.view(heap)?;
    let total = heap.companion(view.bound().ok_or("total")?)?;
    let error = heap.companion(view.evaluation().ok_or("error")?)?;
    assert_eq!(total.target_stamp(), view.stamp());
    assert_eq!(error.target_stamp(), view.stamp());
    Ok([
        view.region().get(0, 0)?.to_f64(),
        total.region().get(0, 0)?.to_f64(),
        error.region().get(0, 0)?.to_f64(),
    ])
}

/// Exact product and quotient fixtures cover both native precisions and original measures.
/// ´test:integration:quantity-products-exact-oracles´
/// ´claim:matrix:quantity-products-exact-oracles´ Products and quotients retain physical kinds, exact measures, original versions and independent rational enclosure targets.
#[test]
fn quantity_products_exact_oracles() -> TestResult {
    physical::<f32>()?;
    physical::<f64>()
}

/// Run the stated 2 V times 3 A and 6 W divided by 2 A oracles in one native format.
fn physical<P: Scalar + 'static>() -> TestResult {
    let (context, [voltage, current, power]) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let left = bounded(
        &context,
        &mut heap,
        &voltage,
        P::from_u8(2),
        P::ZERO,
        P::ZERO,
    )?;
    let right = bounded(
        &context,
        &mut heap,
        &current,
        P::from_u8(3),
        P::ZERO,
        P::ZERO,
    )?;
    let product = context.multiply(
        &mut heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    check_result(&heap, &product, &power, 6)?;
    let record = product.operation_record(&heap)?;
    assert_eq!(record.inputs().len(), 2);
    assert_eq!(record.inputs()[0].stamp(), left.view(&heap)?.stamp());
    assert_eq!(record.inputs()[1].stamp(), right.view(&heap)?.stamp());
    assert_eq!(
        record.operation().inputs()[0].descriptor(),
        left.descriptor()
    );
    assert_eq!(
        record.operation().inputs()[1].descriptor(),
        right.descriptor()
    );
    assert_eq!(evidence(&heap, &product)?.certificates().len(), 9);
    let numerator = bounded(&context, &mut heap, &power, P::from_u8(6), P::ZERO, P::ZERO)?;
    let denominator = bounded(
        &context,
        &mut heap,
        &current,
        P::from_u8(2),
        P::ZERO,
        P::ZERO,
    )?;
    let quotient = context.divide(
        &mut heap,
        &numerator,
        &denominator,
        &mut Budget::new(Limits::default())?,
    )?;
    check_result(&heap, &quotient, &voltage, 3)?;
    assert_eq!(evidence(&heap, &quotient)?.certificates().len(), 12);
    assert_eq!(
        evidence(&heap, &quotient)?
            .certificates()
            .iter()
            .filter(|record| matches!(record.law, KernelLaw::Reciprocal { .. }))
            .count(),
        3
    );
    heap.compact()?;
    check_result(&heap, &product, &power, 6)
}

/// Check a center and both companion enclosures against independent integer truth.
fn check_result<P: Scalar + 'static>(
    heap: &Heap,
    result: &Quantity<P>,
    descriptor: &Descriptor,
    exact: i128,
) -> TestResult {
    assert_eq!(result.descriptor().kind(), descriptor.kind());
    assert_eq!(result.descriptor().dimension(), descriptor.dimension());
    assert_eq!(
        result.descriptor().measure().scale(),
        descriptor.measure().scale()
    );
    let [center, total, error] = channels(heap, result)?;
    assert!(Rational::new(exact, 1)?.covered(center, error)?);
    assert!(Rational::new(exact, 1)?.covered(center, total)?);
    assert!(error <= total);
    Ok(())
}

/// Scaled coordinate products and quotients never normalize either source implicitly.
/// ´test:integration:quantity-products-measure-oracles´
/// ´claim:matrix:quantity-products-measure-oracles´ Noncoherent product and quotient scales compose exactly while native coordinate arithmetic stays unchanged.
#[test]
fn quantity_products_measure_oracles() -> TestResult {
    let (context, [voltage, current, power]) = fixture()?;
    let scaled = Descriptor::new(
        voltage.kind().clone(),
        crate::conversion_testing::measure(1, 1000, 0, 1)?,
        Role::Difference,
    )?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let left = bounded(&context, &mut heap, &scaled, 2.0_f64, 0.0, 0.0)?;
    let right = bounded(&context, &mut heap, &current, 3.0, 0.0, 0.0)?;
    let result = context.multiply(
        &mut heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(result.descriptor().kind(), power.kind());
    assert_eq!(
        result.descriptor().measure().scale(),
        scaled.measure().scale()
    );
    assert_eq!(result.descriptor().measure().name(), "(fixture*current)");
    assert_eq!(channels(&heap, &result)?[0], 6.0);
    let divided = context.divide(
        &mut heap,
        &result,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(divided.descriptor().kind(), voltage.kind());
    assert_eq!(
        divided.descriptor().measure().scale(),
        scaled.measure().scale()
    );
    assert!(
        Rational::new(2, 1)?
            .covered(channels(&heap, &divided)?[0], channels(&heap, &divided)?[2])?
    );
    Ok(())
}

/// Full Cartesian boxes include interaction, and numerical-only targets survive another product.
/// ´test:integration:quantity-products-companion-oracles´
/// ´claim:matrix:quantity-products-companion-oracles´ Product and quotient bounds contain every rational endpoint combination and preserve prior numerical error without double counting it.
#[test]
fn quantity_products_companion_oracles() -> TestResult {
    let (context, _) = fixture()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let left = bounded(
        &context,
        &mut heap,
        &Descriptor::ratio(),
        3.0_f64,
        0.25,
        0.125,
    )?;
    let right = bounded(
        &context,
        &mut heap,
        &Descriptor::ratio(),
        2.0_f64,
        0.25,
        0.125,
    )?;
    for divide in [false, true] {
        let result = if divide {
            context.divide(
                &mut heap,
                &left,
                &right,
                &mut Budget::new(Limits::default())?,
            )?
        } else {
            context.multiply(
                &mut heap,
                &left,
                &right,
                &mut Budget::new(Limits::default())?,
            )?
        };
        let [center, total, error] = channels(&heap, &result)?;
        for (denominator, first, second, radius) in
            [(4, [11, 13], [7, 9], total), (8, [23, 25], [15, 17], error)]
        {
            for first in first {
                for second in second {
                    let first = Rational::new(first, denominator)?;
                    let second = Rational::new(second, denominator)?;
                    let exact = if divide {
                        first.divide(second)?
                    } else {
                        first.multiply(second)?
                    };
                    assert!(exact.covered(center, radius)?);
                }
            }
        }
        assert!(error <= total);
        let one = bounded(&context, &mut heap, &Descriptor::ratio(), 1.0, 0.0, 0.0)?;
        let repeated = context.multiply(
            &mut heap,
            &result,
            &one,
            &mut Budget::new(Limits::default())?,
        )?;
        assert!(channels(&heap, &repeated)?[2] >= error);
    }
    Ok(())
}

/// Borrow the executed certificates from the ordinary current operation record.
fn evidence<'h, P: Scalar + 'static>(
    heap: &'h Heap,
    quantity: &Quantity<P>,
) -> TestResult<&'h nrz_matrix::QuantityKernelEvidence> {
    quantity
        .operation_record(heap)?
        .evidence()
        .iter()
        .find_map(|claim| {
            if let nrz_matrix::RecipeEvidence::QuantityKernel(claim) = claim {
                Some(claim.as_ref())
            } else {
                None
            }
        })
        .ok_or_else(|| "quantity kernel evidence".into())
}

/// Additive boxed quantity evidence preserves the measured seed inline layouts.
/// ´test:integration:quantity-evidence-layout-boundary´
/// ´claim:matrix:quantity-evidence-layout-boundary´ `RecipeEvidence` and `KernelEvidence` retain their literal 32-byte and 24-byte seed layouts.
#[test]
fn quantity_evidence_layout_boundary() {
    assert_eq!(
        (
            size_of::<nrz_matrix::RecipeEvidence>(),
            size_of::<nrz_matrix::KernelEvidence>()
        ),
        (32, 24)
    );
    assert_eq!(
        (
            size_of::<nrz_matrix::QuantityKernelEvidence>(),
            size_of::<nrz_scalar::KernelCertificate>()
        ),
        (64, 104)
    );
}

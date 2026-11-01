// SPDX-License-Identifier: AGPL-3.0-only
//! Explicit quantity frequency relations retain original targets and certified coefficient error.

mod boundaries;
mod covariance;
mod refusals;

use super::pi_conversion::{exact_policy, oracle, pi};
use super::quantity_products::{bounded, channels};
use crate::{conversion_testing::TestResult, kernel_oracle::Rational};
use nrz_bindings::{
    Budget, Context, Descriptor, ExponentPolicy, Kind, Limits, Measure, Registry, Role,
};
use nrz_matrix::{
    ExecutionLimits, Heap, HeapLimits, PiConstant, Quantity, QuantityContext, ShapeLimits,
};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// Fresh declaration authority and independent source/companion storage for every attempt.
struct Case<P: Scalar> {
    context: QuantityContext,
    heap: Heap,
    input: Quantity<P>,
    circle: PiConstant<P>,
    target: Descriptor,
}

/// Exact coherent frequency descriptors retain their distinct kinds.
fn descriptor(angular: bool) -> TestResult<Descriptor> {
    Ok(Descriptor::new(
        if angular {
            Kind::angular_frequency()?
        } else {
            Kind::frequency()?
        },
        Measure::coherent(),
        Role::Difference,
    )?)
}

/// One explicit full-cycle registry and a caller-owned numerical pi region.
fn fixture<P: Scalar + 'static>(represented: bool) -> TestResult<Case<P>> {
    let owner = Context::with_real_symbols(
        Registry::with_angle_frequency(ExponentPolicy::Rational)?,
        Limits::default(),
        &[],
    )?
    .freeze()?;
    let context = QuantityContext::new(
        Rc::new(owner),
        ShapeLimits::default(),
        ExecutionLimits::default(),
    );
    let mut heap = Heap::new(HeapLimits::default())?;
    let source = descriptor(true)?;
    let center = P::from_u8(200) * P::from_u8(10);
    let input = if represented {
        let source = heap.import_represented(nrz_matrix::Extent::new(1, 1)?, &source, &[center])?;
        context.capture(
            &heap,
            heap.symbol_binding(source.value())?,
            &mut Budget::new(Limits::default())?,
        )?
    } else {
        bounded(
            &context,
            &mut heap,
            &source,
            center,
            P::ONE,
            P::ONE / P::from_u8(8),
        )?
    };
    let circle = pi::<P>(
        &ArithmeticContext::new()?,
        &mut Budget::new(Limits::default())?,
    )?;
    Ok(Case {
        context,
        heap,
        input,
        circle,
        target: descriptor(false)?,
    })
}

/// Execute one actual adapter attempt with its complete caller ledger.
fn convert<P: Scalar + 'static>(
    case: &mut Case<P>,
    budget: &mut Budget,
) -> Result<Quantity<P>, nrz_matrix::QuantityError<P>> {
    case.context.convert_frequency(
        &mut case.heap,
        &case.input,
        &case.target,
        &case.circle,
        &exact_policy(),
        budget,
    )
}

/// Independently enclose the exact full-cycle reference and inverse with prior error in both formats.
/// ´test:integration:quantity-frequency-reference-roundtrip´
#[test]
fn quantity_frequency_reference_roundtrip() -> TestResult {
    roundtrip::<f32>()?;
    roundtrip::<f64>()
}

/// The independent atan sum has no dependency on the production Machin pi primitive.
fn roundtrip<P: Scalar + 'static>() -> TestResult {
    let mut case = fixture::<P>(false)?;
    let cyclic = convert(&mut case, &mut Budget::new(Limits::default())?)?;
    let [center, total, error] = channels(&case.heap, &cyclic)?;
    oracle::covers(oracle::factor(1000, 1, true, 48)?, center, error)?;
    oracle::covers(
        [
            oracle::factor(1999, 2, true, 48)?[0],
            oracle::factor(2001, 2, true, 48)?[1],
        ],
        center,
        total,
    )?;
    oracle::covers(
        [
            oracle::factor(15_999, 16, true, 48)?[0],
            oracle::factor(16_001, 16, true, 48)?[1],
        ],
        center,
        error,
    )?;
    assert!(error <= total);
    assert!(!Rational::new(-318, 1)?.covered(center, total)?);
    assert!(!Rational::new(636, 1)?.covered(center, total)?);
    let restored = case.context.convert_frequency(
        &mut case.heap,
        &cyclic,
        case.input.descriptor(),
        &case.circle,
        &exact_policy(),
        &mut Budget::new(Limits::default())?,
    )?;
    let [center, total, error] = channels(&case.heap, &restored)?;
    oracle::covers(
        [Rational::new(1999, 1)?, Rational::new(2001, 1)?],
        center,
        total,
    )?;
    oracle::covers(
        [Rational::new(15_999, 8)?, Rational::new(16_001, 8)?],
        center,
        error,
    )?;
    assert_eq!(
        restored.view(&case.heap)?.coverage(),
        nrz_matrix::Coverage::DeclaredBounds
    );
    Ok(())
}

/// Represented inputs gain coefficient error without acquiring physical-bound coverage.
/// ´test:integration:quantity-frequency-represented-error´
#[test]
fn quantity_frequency_represented_error() -> TestResult {
    represented::<f32>()?;
    represented::<f64>()
}

fn represented<P: Scalar + 'static>() -> TestResult {
    let mut case = fixture::<P>(true)?;
    let output = convert(&mut case, &mut Budget::new(Limits::default())?)?;
    let [center, total, error] = channels(&case.heap, &output)?;
    oracle::covers(oracle::factor(1000, 1, true, 48)?, center, error)?;
    assert!(error > 0.0);
    assert_eq!(total, error);
    assert_eq!(
        output.view(&case.heap)?.coverage(),
        nrz_matrix::Coverage::RepresentedInputs
    );
    let original = case.circle.evidence();
    drop(case.circle);
    let nrz_matrix::RecipeEvidence::PiConversion(evidence) = output
        .operation_record(&case.heap)?
        .evidence()
        .first()
        .ok_or("evidence")?
    else {
        return Err("pi evidence".into());
    };
    assert!(evidence.constant().same_region(&original));
    Ok(())
}

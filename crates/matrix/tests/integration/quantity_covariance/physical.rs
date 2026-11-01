// SPDX-License-Identifier: AGPL-3.0-only
//! Physical quantity products retain declared kind relations as well as correlation.

mod frequency;
mod measurements;

use super::{TestResult, append_pair};
use crate::integration::quantity_products::descriptor;
use crate::kernel_oracle::Rational;
use crate::statistics_testing::check;
use nrz_bindings::{BinaryOp, Budget, Context, ExponentPolicy, Limits, Registry};
use nrz_matrix::{
    ExecutionLimits, Heap, HeapLimits, MomentAssurance, QuantityContext, ShapeLimits,
};
use std::rc::Rc;

/// The physical LC product is the first retained stage of the resonance law.
/// ´test:integration:quantity-product-covariance-lc´
/// ´claim:matrix:quantity-product-covariance-lc´ Correlated inductance and capacitance retain the explicitly registered time-squared product kind, exact coordinate pairings and rational variance `37/1024`.
#[test]
fn quantity_product_covariance_lc() -> TestResult {
    let inductance = descriptor("inductance", [2, 1, -2, -2, 0, 0, 0], (1, 1))?;
    let capacitance = descriptor("capacitance", [-2, -1, 4, 2, 0, 0, 0], (1, 1))?;
    let squared_time = descriptor("squared time", [0, 0, 2, 0, 0, 0, 0], (1, 1))?;
    let mut registry = Registry::new(ExponentPolicy::Rational);
    registry.register_binary(
        BinaryOp::Multiply,
        inductance.kind().clone(),
        capacitance.kind().clone(),
        squared_time.kind().clone(),
    )?;
    let owner = Context::with_real_symbols(registry, Limits::default(), &[])?.freeze()?;
    let context = QuantityContext::new(
        Rc::new(owner),
        ShapeLimits::default(),
        ExecutionLimits::default(),
    );
    let mut case = append_pair(
        context,
        Heap::new(HeapLimits::default())?,
        [&inductance, &capacitance],
        [0.25_f64, 0.0625],
    )?;
    let product = case.context.multiply(
        &mut case.heap,
        &case.values[0],
        &case.values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(product.descriptor().kind(), squared_time.kind());
    assert_eq!(product.view(&case.heap)?.region().get(0, 0)?, 0.015_625);
    let target = case
        .heap
        .companion(product.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = case.heap.moments(case.heap.moment_companion(target)?)?;
    assert_eq!(moments.assurance(), MomentAssurance::LinearizedMoments);
    check(
        moments.jacobian().ok_or("Jacobian")?,
        &[Rational::new(1, 16)?, Rational::new(1, 4)?],
    )?;
    check(moments.covariance(), &[Rational::new(37, 1024)?])?;
    Ok(())
}

/// The angular-frequency reading retains its exact radian operand and correlated L/C model.
/// ´test:integration:quantity-root-covariance-lc-resonance´
/// ´claim:matrix:quantity-root-covariance-lc-resonance´ The checked LC product, positive root, reciprocal power and explicit radian product retain one joint model and enclose angular frequency eight with source derivatives minus sixteen and minus sixty-four and variance 2368.
#[test]
fn quantity_root_covariance_lc_resonance() -> TestResult {
    let (context, mut heap, values) = resonance_case()?;
    let product = context.multiply(
        &mut heap,
        &values[0],
        &values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    let root = context.square_root(&mut heap, &product, &mut Budget::new(Limits::default())?)?;
    let inverse =
        context.integer_power(&mut heap, &root, -1, &mut Budget::new(Limits::default())?)?;
    let omega = context.multiply(
        &mut heap,
        &inverse,
        &values[2],
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(
        omega.descriptor().kind(),
        &nrz_bindings::Kind::angular_frequency()?
    );
    assert_eq!(omega.view(&heap)?.region().get(0, 0)?, 8.0);
    let target = heap
        .companion(omega.view(&heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = heap.moments(heap.moment_companion(target)?)?;
    check(
        moments.sensitivity().ok_or("sensitivity")?,
        &[
            Rational::new(-16, 1)?,
            Rational::new(-64, 1)?,
            Rational::new(8, 1)?,
        ],
    )?;
    let variance = Rational::new(256, 1)?
        .multiply(Rational::new(1, 4)?)?
        .add(Rational::new(4096, 1)?.multiply(Rational::new(1, 2)?)?)?
        .add(Rational::new(2048, 1)?.multiply(Rational::new(1, 8)?)?)?;
    check(moments.covariance(), &[variance])?;
    assert_eq!((variance.numerator, variance.denominator), (2368, 1));
    assert_eq!(moments.assurance(), MomentAssurance::LinearizedMoments);
    assert!(moments.means().is_none());
    assert_eq!(moments.source_ids().len(), 3);
    Ok(())
}

/// Register each physical relation, including the explicit radian product.
fn resonance_context(
    frequency: bool,
) -> TestResult<(QuantityContext, [nrz_bindings::Descriptor; 3])> {
    use nrz_bindings::readings::algebra::Rational as Exact;
    use nrz_bindings::{Descriptor, Kind, Measure, Role};
    let inductance = descriptor("inductance", [2, 1, -2, -2, 0, 0, 0], (1, 1))?;
    let capacitance = descriptor("capacitance", [-2, -1, 4, 2, 0, 0, 0], (1, 1))?;
    let squared_time = descriptor("squared time", [0, 0, 2, 0, 0, 0, 0], (1, 1))?;
    let time = descriptor("time", [0, 0, 1, 0, 0, 0, 0], (1, 1))?;
    let angle = Descriptor::new(Kind::Angle, Measure::coherent(), Role::Difference)?;
    let mut registry = if frequency {
        Registry::with_angle_frequency(ExponentPolicy::Rational)?
    } else {
        Registry::new(ExponentPolicy::Rational)
    };
    let mut budget = Budget::new(Limits::default())?;
    registry.register_binary(
        BinaryOp::Multiply,
        inductance.kind().clone(),
        capacitance.kind().clone(),
        squared_time.kind().clone(),
    )?;
    registry.register_power(
        squared_time.kind().clone(),
        Exact::from_ratio(1, 2, &mut budget)?,
        time.kind().clone(),
    )?;
    registry.register_power(
        time.kind().clone(),
        Exact::from_ratio(2, 1, &mut budget)?,
        squared_time.kind().clone(),
    )?;
    registry.register_power(
        time.kind().clone(),
        Exact::from_ratio(-1, 1, &mut budget)?,
        Kind::rate()?,
    )?;
    if frequency {
        let period = descriptor("period", [0, 0, 1, 0, 0, 0, 0], (1, 1))?;
        let reciprocal = Exact::from_ratio(-1, 1, &mut budget)?;
        registry.register_power(
            Kind::frequency()?,
            reciprocal.clone(),
            period.kind().clone(),
        )?;
        registry.register_power(period.kind().clone(), reciprocal, Kind::frequency()?)?;
    } else {
        registry.register_binary(
            BinaryOp::Multiply,
            Kind::rate()?,
            Kind::Angle,
            Kind::angular_frequency()?,
        )?;
    }
    let context = QuantityContext::new(
        Rc::new(Context::with_real_symbols(registry, Limits::default(), &[])?.freeze()?),
        ShapeLimits::default(),
        ExecutionLimits::default(),
    );
    Ok((context, [inductance, capacitance, angle]))
}

/// Three original sources include an exact radian with declared zero covariance.
fn resonance_case() -> TestResult<(QuantityContext, Heap, [nrz_matrix::Quantity<f64>; 3])> {
    resonance_at([0.25_f64, 0.0625], [0.5, 0.25, 7.0])
}

/// Original coherent L/C declarations and a source-free exact radian share one joint model.
fn resonance_at<P: nrz_scalar::Scalar + 'static>(
    centers: [P; 2],
    covariance: [P; 3],
) -> TestResult<(QuantityContext, Heap, [nrz_matrix::Quantity<P>; 3])> {
    resonance_at_with(centers, covariance, false)
}

/// The frequency fixture explicitly adds the full-cycle vocabulary; existing fixture costs stay fixed.
fn resonance_at_with<P: nrz_scalar::Scalar + 'static>(
    centers: [P; 2],
    covariance: [P; 3],
    frequency: bool,
) -> TestResult<(QuantityContext, Heap, [nrz_matrix::Quantity<P>; 3])> {
    use nrz_matrix::{
        CompanionSetDraft, CovarianceBlock, Extent, JointLimits, JointModel, Source, SourceFactor,
        TargetMap,
    };
    let (context, [inductance, capacitance, angle]) = resonance_context(frequency)?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let descriptors = [&inductance, &capacitance, &angle];
    let handles = descriptors
        .into_iter()
        .zip([centers[0], centers[1], P::ONE])
        .map(|(descriptor, center)| -> TestResult<_> {
            Ok(heap
                .import_represented(Extent::new(1, 1)?, descriptor, &[center])?
                .value())
        })
        .collect::<TestResult<Vec<_>>>()?;
    let sources = descriptors
        .into_iter()
        .map(|descriptor| Source::new(descriptor, 1))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = JointLimits::default();
    let factor = SourceFactor::new(
        3,
        &[
            covariance[0],
            covariance[1],
            P::ZERO,
            P::ZERO,
            covariance[1],
            P::ZERO,
        ],
        &[P::ONE, covariance[2]],
        limits,
    )?;
    let model = JointModel::from_factor(
        &sources,
        factor,
        &[const { CovarianceBlock::Known }; 9],
        limits,
    )?;
    for (left, right) in [(0, 1), (1, 2)] {
        heap.publish_joint(
            CompanionSetDraft::new(
                model.clone(),
                [
                    TargetMap::capture(&heap, handles[left])?,
                    TargetMap::capture(&heap, handles[right])?,
                ],
                [vec![left], vec![right]],
            ),
            limits,
        )?;
    }
    let mut values = handles
        .into_iter()
        .map(|handle| {
            context
                .capture(
                    &heap,
                    heap.symbol_binding(handle)?,
                    &mut Budget::new(Limits::default())?,
                )
                .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let angle = values.pop().ok_or("angle")?;
    let capacitance = values.pop().ok_or("capacitance")?;
    let inductance = values.pop().ok_or("inductance")?;
    Ok((context, heap, [inductance, capacitance, angle]))
}

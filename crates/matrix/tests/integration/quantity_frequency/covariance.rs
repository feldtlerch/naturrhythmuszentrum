// SPDX-License-Identifier: AGPL-3.0-only
//! Both ordered covariance axes and inherited local assurance survive explicit frequency conversion.

use super::{TestResult, convert, fixture, oracle};
use crate::kernel_oracle::Rational;
use nrz_bindings::{Budget, Limits, PairingWitness, UncertaintyMode};
use nrz_matrix::{JointLimits, MomentAssurance, ParameterView};
use nrz_scalar::Scalar;

/// Conversion preserves local assurance and scales both self and cross covariance axes.
/// ´test:integration:quantity-frequency-local-covariance´
#[test]
fn quantity_frequency_local_covariance() -> TestResult {
    covariance::<f32>()?;
    covariance::<f64>()
}

fn covariance<P: Scalar + 'static>() -> TestResult {
    let mut case = fixture::<P>(false)?;
    let source = case
        .heap
        .companion(case.input.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    crate::statistics_testing::attach(
        &mut case.heap,
        source,
        &[P::from_u8(7) * P::from_u8(100)],
        true,
    )?;
    // A local nonlinear recipe supplies inherited local assurance; its input mean must not reappear.
    let omega = case.context.integer_power(
        &mut case.heap,
        &case.input,
        1,
        &mut Budget::new(Limits::default())?,
    )?;
    case.input = omega;
    let angular = case
        .heap
        .companion(case.input.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let original = case.heap.moments(case.heap.moment_companion(angular)?)?;
    let model = original.model();
    let ids = original.source_ids().to_vec();
    assert_eq!(original.assurance(), MomentAssurance::LinearizedMoments);
    assert!(original.means().is_none());
    let cyclic = convert(&mut case, &mut Budget::new(Limits::default())?)?;
    let target = case
        .heap
        .companion(cyclic.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moment = case.heap.moments(case.heap.moment_companion(target)?)?;
    assert_eq!(moment.model(), model);
    assert_eq!(moment.source_ids(), ids);
    assert_eq!(moment.assurance(), MomentAssurance::LinearizedMoments);
    assert!(moment.means().is_none());
    let factor = oracle::factor(1, 2, true, 56)?;
    check_parameter(moment.jacobian().ok_or("Jacobian")?, factor)?;
    check_parameter(moment.sensitivity().ok_or("source sensitivity")?, factor)?;
    let variance = Rational::new(700, 1)?;
    check_parameter(
        moment.covariance(),
        [
            factor[0].multiply(factor[0])?.multiply(variance)?,
            factor[1].multiply(factor[1])?.multiply(variance)?,
        ],
    )?;
    for (left, right) in [(angular, target), (target, angular)] {
        let cross = case
            .heap
            .materialize_cross_covariance(left, right, JointLimits::default())?;
        let view = case.heap.moments(cross)?;
        check_parameter(
            view.covariance(),
            [factor[0].multiply(variance)?, factor[1].multiply(variance)?],
        )?;
        assert_eq!(view.model(), model);
        assert_eq!(view.assurance(), MomentAssurance::LinearizedMoments);
        let left = case.heap.symbol_binding(left)?;
        let right = case.heap.symbol_binding(right)?;
        let paired = PairingWitness::uncertainty(
            UncertaintyMode::Covariance,
            left.symbol().descriptor(),
            Some(right.symbol().descriptor()),
            None,
        )?;
        assert_eq!(view.covariance().descriptor(0, 0)?, &paired);
        assert_eq!(view.covariance().error_descriptor(0, 0)?.target(), &paired);
    }
    let stamp = case.heap.value(source)?.stamp();
    let center = case.heap.value(source)?.region().get(0, 0)?;
    case.heap
        .replace_represented(source, stamp.version(), &[center])?;
    assert!(cyclic.view(&case.heap).is_err());
    Ok(())
}

fn check_parameter<P: Scalar + 'static>(
    view: &ParameterView<'_, P>,
    bounds: [Rational; 2],
) -> TestResult {
    oracle::covers(bounds, view.get(0, 0)?.to_f64(), view.error(0, 0)?.to_f64())
}

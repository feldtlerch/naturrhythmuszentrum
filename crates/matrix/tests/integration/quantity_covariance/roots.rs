// SPDX-License-Identifier: AGPL-3.0-only
//! Positive quantity roots keep polynomial identities and a separate numerical domain.

use super::{TestResult, append_pair};
use crate::kernel_oracle::Rational;
use crate::kernel_testing::owner;
use crate::statistics_testing::check;
use nrz_bindings::{Budget, Descriptor, Limits, SensitivityRecipe};
use nrz_matrix::{
    ExecutionLimits, Heap, HeapLimits, MomentAssurance, QuantityContext, ShapeLimits,
};
use nrz_scalar::Scalar;

/// An exact root point provides a rational oracle independent of the production schedule.
/// ´test:integration:quantity-root-covariance-oracle´
/// ´claim:matrix:quantity-root-covariance-oracle´ Positive quantity roots enclose the rational derivative one quarter and variance one sixty-fourth, retain their polynomial proof and report linearized moments without an output mean in both formats.
#[test]
fn quantity_root_covariance_oracle() -> TestResult {
    oracle::<f32>()?;
    oracle::<f64>()
}

/// At four the positive root is two, so `J=1/4` and `J^2*U=1/64`.
fn oracle<P: Scalar + 'static>() -> TestResult {
    let ratio = Descriptor::ratio();
    let context =
        QuantityContext::new(owner()?, ShapeLimits::default(), ExecutionLimits::default());
    let mut case = append_pair(
        context,
        Heap::new(HeapLimits::default())?,
        [&ratio, &ratio],
        [P::from_u8(4), P::from_u8(2)],
    )?;
    let result = case.context.square_root(
        &mut case.heap,
        &case.values[0],
        &mut Budget::new(Limits::default())?,
    )?;
    let target = case
        .heap
        .companion(result.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = case.heap.moments(case.heap.moment_companion(target)?)?;
    check(
        moments.jacobian().ok_or("Jacobian")?,
        &[Rational::new(1, 4)?],
    )?;
    check(moments.covariance(), &[Rational::new(1, 64)?])?;
    assert_eq!(moments.assurance(), MomentAssurance::LinearizedMoments);
    assert!(moments.means().is_none());
    let proof = moments.checked_sensitivity().ok_or("proof")?;
    assert_eq!(proof.recipe(), SensitivityRecipe::QuantityPositiveRoot);
    assert_eq!(proof.outputs(), std::slice::from_ref(result.descriptor()));
    assert_eq!(proof.jacobian().witnesses().len(), 2);
    assert_eq!(proof.auxiliary_symbols(), &[String::from("t")]);
    assert!(
        proof
            .jacobian()
            .entries()
            .iter()
            .all(|entry| entry.identity().is_discharged())
    );
    Ok(())
}

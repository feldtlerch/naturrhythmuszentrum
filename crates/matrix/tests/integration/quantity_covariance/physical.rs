// SPDX-License-Identifier: AGPL-3.0-only
//! Physical quantity products retain declared kind relations as well as correlation.

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

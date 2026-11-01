// SPDX-License-Identifier: AGPL-3.0-only
//! Native connected frequency publication keeps one original LC evaluation and its complete model.

mod channels;
mod costs;
mod definition;
mod lifecycle;
mod models;
mod numeric;
mod storage;
mod symbols;

use super::{reciprocal_statistics as shared, root_support as inputs};
use crate::{reading_testing::TestResult, spectral_testing::Fixture};
use nrz_bindings::{
    Budget, CheckedDeclarations, Context, ExponentPolicy, Limits, Registry,
    readings::LcFrequencyDefinition,
};
use nrz_matrix::{
    ConversionContract, ConversionMode, LcFrequencyPublication, ReciprocalDerivativeContract,
};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};
use std::rc::Rc;

struct Prepared {
    fixture: Fixture,
    derivative: ReciprocalDerivativeContract,
    definition: LcFrequencyDefinition,
    declarations: Rc<CheckedDeclarations>,
}

fn prepared() -> TestResult<Prepared> {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_store_limits(["5", "1/10", "1/2000"], budget.limits().clone())?;
    let derivative = shared::derivative(&mut fixture, &mut budget)?;
    let declarations = Rc::new(
        Context::with_real_symbols(
            Registry::with_angle_frequency(ExponentPolicy::Rational)?,
            Limits::default(),
            &[],
        )?
        .freeze()?,
    );
    let graph = Rc::get_mut(&mut fixture.graph).ok_or("unique frequency graph")?;
    let definition = symbols::prepare(graph, fixture.law.clone(), &declarations, &mut budget)?;
    Ok(Prepared {
        fixture,
        derivative,
        definition,
        declarations,
    })
}

fn publication<P: Scalar + 'static>(
    prepared: &Prepared,
    case: &inputs::Case<P>,
    budget: &mut Budget,
) -> TestResult<LcFrequencyPublication<P>> {
    let context = ArithmeticContext::new()?;
    let plan = inputs::plan(&prepared.fixture, case, budget)?;
    let evaluation = case.heap.evaluate_lc_spectrum(&plan, &context, budget)?;
    let conversion = conversion(prepared)?;
    let pi = crate::integration::pi_conversion::pi::<P>(&context, budget)?;
    Ok(LcFrequencyPublication::new(
        evaluation,
        prepared.definition.clone(),
        &conversion,
        &pi,
        &crate::integration::pi_conversion::exact_policy(),
        &context,
        budget,
    )?)
}

fn conversion(prepared: &Prepared) -> TestResult<ConversionContract> {
    let definition = &prepared.definition;
    Ok(ConversionContract::new(
        crate::conversion_testing::shape(
            &prepared.declarations,
            definition.angular().descriptor(),
            1,
        )?,
        crate::conversion_testing::shape(&prepared.declarations, definition.descriptor(), 1)?,
        definition.map().clone(),
        LiteralLimits::default(),
        ConversionMode::Deterministic,
    )?)
}

/// Both frequencies and the complete covariance matrix retain the original root, model and point.
/// ´test:integration:lc-frequency-connected-original-output´
#[test]
fn lc_frequency_connected_original_output() -> TestResult {
    let prepared = prepared()?;
    original::<f32>(&prepared)?;
    original::<f64>(&prepared)
}

fn original<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut budget)?;
    let plan = publication(prepared, &case, &mut budget)?;
    let usage = plan.evaluation().usage();
    let context = ArithmeticContext::new()?;
    let output = case
        .heap
        .publish_lc_frequency(&plan, &context, &mut budget)?;
    assert_eq!(output.evaluation(&case.heap, &context)?.usage(), usage);
    assert_eq!(
        output.operating_point(&case.heap, &context)?,
        [P::ONE / P::from_u8(4), P::ONE / P::from_u8(16)]
    );
    assert_eq!(output.readings().bindings().len(), 5);
    assert_eq!(output.readings().bindings()[0].companions().len(), 7);
    assert_eq!(
        output.source_model(&case.heap, &context)?.model().id(),
        case.heap.statistical(case.companion)?.model().id()
    );
    assert_eq!(
        output.assurance(&case.heap, &context)?,
        nrz_matrix::MomentAssurance::LinearizedMoments
    );
    let values = output.values(&case.heap, &context)?;
    crate::integration::pi_conversion::oracle::covers(
        crate::integration::pi_conversion::oracle::factor(4, 1, true, 56)?,
        values[1].center.to_f64(),
        values[1].error.to_f64(),
    )?;
    assert!(values[1].error > P::ZERO);
    assert!(values[1].error <= values[1].total);
    case.heap.compact()?;
    output.covariance(&case.heap, &context)?;
    assert!(
        case.heap
            .publish_lc_frequency(&plan, &context, &mut budget)
            .is_err()
    );
    Ok(())
}

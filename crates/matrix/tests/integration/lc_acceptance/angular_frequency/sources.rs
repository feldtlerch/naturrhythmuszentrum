// SPDX-License-Identifier: AGPL-3.0-only
//! Complete declared source factors distinguish correlation, common fluctuations and fixed inputs.

use super::super::selected_joint;
use super::{Prepared, TestResult, inputs, prepared, publication, shared};
use nrz_bindings::Budget;
use nrz_matrix::{
    CompanionSetDraft, CovarianceBlock, JointLimits, JointModel, Source, SourceFactor,
};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Import one complete original source model without treating a fixed input as missing statistics.
pub(super) fn case<P: Scalar + 'static>(
    prepared: &Prepared,
    values: [[P; 3]; 2],
    factor: &[P],
    weights: &[P],
    cross: CovarianceBlock,
    budget: &mut Budget,
) -> TestResult<inputs::Case<P>> {
    let (mut heap, original) = prepared.fixture.import_with_budget(values, budget)?;
    let selected = original
        .iter()
        .map(|source| heap.select_reading(source, budget))
        .collect::<Result<Vec<_>, _>>()?;
    let handles = selected_joint::capture_handles(&mut heap, &selected)?;
    let sources = selected
        .iter()
        .map(|value| Source::new(value.symbol().descriptor(), 1))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = JointLimits::default();
    let model = JointModel::from_factor(
        &sources,
        SourceFactor::new(2, factor, weights, limits)?,
        &[
            CovarianceBlock::Known,
            cross.clone(),
            cross,
            CovarianceBlock::Known,
        ],
        limits,
    )?;
    let companion = heap.publish_joint(
        CompanionSetDraft::new(
            model,
            selected_joint::capture_targets(&heap, handles)?,
            [vec![0], vec![1]],
        ),
        limits,
    )?;
    let derivative = heap.evaluate_reciprocal_derivative(
        &prepared.derivative,
        &original,
        &ArithmeticContext::new()?,
        budget,
    )?;
    Ok(inputs::Case {
        heap,
        original,
        selected,
        derivative,
        companion,
    })
}

/// ´test:integration:lc-angular-frequency-common-and-fixed-source-twins´
#[test]
fn lc_angular_frequency_common_and_fixed_source_twins() -> TestResult {
    let prepared = prepared()?;
    twins::<f32>(&prepared)?;
    twins::<f64>(&prepared)
}

fn twins<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    use crate::reciprocal_testing::oracle::Rational;
    let half = P::ONE / P::from_u8(2);
    let quarter = P::ONE / P::from_u8(4);
    for (factor, variance, zero) in [
        ([half, quarter, P::ZERO, P::ZERO], 576, false),
        ([half, P::ZERO, P::ZERO, quarter], 320, true),
        ([half, P::ZERO, P::ZERO, P::ZERO], 64, true),
        ([P::ZERO; 4], 0, true),
    ] {
        let cross = if zero {
            CovarianceBlock::DeclaredZero("explicit zero cross covariance".into())
        } else {
            CovarianceBlock::Known
        };
        let mut budget = shared::policy()?;
        let mut case = case(
            prepared,
            [
                [quarter, P::ZERO, P::ZERO],
                [P::ONE / P::from_u8(16), P::ZERO, P::ZERO],
            ],
            &factor,
            &[P::ONE; 2],
            cross.clone(),
            &mut budget,
        )?;
        let plan = publication(prepared, &case, &mut budget)?;
        let context = ArithmeticContext::new()?;
        let output = case
            .heap
            .publish_lc_angular_frequency(&plan, &context, &mut budget)?;
        let actual = output.covariance(&case.heap, &context)?;
        assert!(
            Rational::new(variance, 1)?.covered(actual.center.to_f64(), actual.error.to_f64())?
        );
        assert_eq!(
            output
                .source_model(&case.heap, &context)?
                .model()
                .block(0, 1)?,
            &cross
        );
        assert_eq!(
            output.assurance(&case.heap, &context)?,
            nrz_matrix::MomentAssurance::LinearizedMoments
        );
    }
    Ok(())
}

/// ´test:integration:lc-angular-frequency-source-model-block-refusals´
#[test]
fn lc_angular_frequency_source_model_block_refusals() -> TestResult {
    let prepared = prepared()?;
    let mut budget = shared::policy()?;
    let case = inputs::nominal::<f64>(&prepared.fixture, &prepared.derivative, &mut budget)?;
    let plan = prepared.fixture.plan(case.original.clone(), &mut budget)?;
    assert!(matches!(
        nrz_matrix::LcAngularFrequencyPublication::new(
            plan.clone(),
            prepared.definition.clone(),
            &mut budget
        ),
        Err(nrz_matrix::SpectralError::Identity)
    ));
    let foreign = inputs::nominal::<f64>(&prepared.fixture, &prepared.derivative, &mut budget)?;
    for (derivative, companion) in [
        (foreign.derivative, case.companion),
        (case.derivative.clone(), foreign.companion),
    ] {
        assert!(
            plan.clone()
                .with_reciprocal_statistics(&case.heap, derivative, companion, &mut budget)
                .is_err()
        );
    }
    let missing = inputs::prepare(
        &prepared.fixture,
        &prepared.derivative,
        [[0.25_f64, 0.0, 0.0], [0.0625, 0.0, 0.0]],
        [0.5, 0.25],
        false,
        &mut budget,
    )?;
    let missing_plan = prepared
        .fixture
        .plan(missing.original.clone(), &mut budget)?;
    assert!(matches!(
        missing_plan.with_reciprocal_statistics(
            &missing.heap,
            missing.derivative,
            missing.companion,
            &mut budget
        ),
        Err(nrz_matrix::SpectralError::Joint(
            nrz_matrix::JointError::MissingCrossBlock
        ))
    ));
    let mut case = case;
    let publication = publication(&prepared, &case, &mut budget)?;
    case.heap.publish_lc_angular_frequency(
        &publication,
        &ArithmeticContext::new()?,
        &mut budget,
    )?;
    Ok(())
}

/// A companion covering only L cannot declare C fixed by omission.
/// ´test:integration:lc-angular-frequency-missing-source-refusal´
#[test]
fn lc_angular_frequency_missing_source_refusal() -> TestResult {
    let prepared = prepared()?;
    missing_source::<f32>(&prepared)?;
    missing_source::<f64>(&prepared)
}

fn missing_source<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    let values = [
        [P::ONE / P::from_u8(4), P::ZERO, P::ZERO],
        [P::ONE / P::from_u8(16), P::ZERO, P::ZERO],
    ];
    let (mut heap, original) = prepared.fixture.import_with_budget(values, &mut budget)?;
    let left = heap.select_reading(&original[0], &mut budget)?;
    let handles = selected_joint::capture_handles(&mut heap, std::slice::from_ref(&left))?;
    let limits = JointLimits::default();
    let model = JointModel::from_factor(
        &[Source::new(left.symbol().descriptor(), 1)?],
        SourceFactor::new(1, &[P::ONE], &[P::ONE], limits)?,
        &[CovarianceBlock::Known],
        limits,
    )?;
    let companion = heap.publish_joint(
        CompanionSetDraft::new(
            model,
            selected_joint::capture_targets(&heap, handles)?,
            [vec![0], vec![0]],
        ),
        limits,
    )?;
    let derivative = heap.evaluate_reciprocal_derivative(
        &prepared.derivative,
        &original,
        &ArithmeticContext::new()?,
        &mut budget,
    )?;
    let plan = prepared.fixture.plan(original, &mut budget)?;
    assert!(matches!(
        plan.with_reciprocal_statistics(&heap, derivative, companion, &mut budget),
        Err(nrz_matrix::SpectralError::Identity)
    ));
    // A fresh complete declaration is the positive twin; no conflicting second model is attached.
    let mut complete = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut budget)?;
    let plan = publication(prepared, &complete, &mut budget)?;
    complete
        .heap
        .publish_lc_angular_frequency(&plan, &ArithmeticContext::new()?, &mut budget)?;
    Ok(())
}

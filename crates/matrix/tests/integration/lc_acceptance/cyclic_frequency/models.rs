// SPDX-License-Identifier: AGPL-3.0-only
//! Connected outputs preserve the original model identity, source rows and zero-block assumptions.

use super::super::selected_joint;
use super::{Prepared, TestResult, inputs, prepared, publication, shared};
use nrz_bindings::Budget;
use nrz_matrix::{
    CompanionSetDraft, CovarianceBlock, Heap, HeapLimits, JointLimits, JointModel, Source,
    SourceFactor,
};
use nrz_scalar::{ArithmeticContext, Scalar};

/// An explicitly complete two-source factor also represents fixed coordinates without omitting them.
pub(super) fn case<P: Scalar + 'static>(
    prepared: &Prepared,
    values: [[P; 3]; 2],
    factor: [P; 4],
    weights: [P; 2],
    cross: CovarianceBlock,
    budget: &mut Budget,
) -> TestResult<inputs::Case<P>> {
    in_heap(
        prepared,
        Heap::new(HeapLimits::default())?,
        values,
        factor,
        weights,
        cross,
        budget,
    )
}

pub(super) fn in_heap<P: Scalar + 'static>(
    prepared: &Prepared,
    mut heap: Heap,
    values: [[P; 3]; 2],
    factor: [P; 4],
    weights: [P; 2],
    cross: CovarianceBlock,
    budget: &mut Budget,
) -> TestResult<inputs::Case<P>> {
    let original = prepared.fixture.import_into(&mut heap, values, budget)?;
    let selected = original
        .iter()
        .map(|reading| heap.select_reading(reading, budget))
        .collect::<Result<Vec<_>, _>>()?;
    let handles = selected_joint::capture_handles(&mut heap, &selected)?;
    let sources = selected
        .iter()
        .map(|value| Source::new(value.symbol().descriptor(), 1))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = JointLimits::default();
    let model = JointModel::from_factor(
        &sources,
        SourceFactor::new(2, &factor, &weights, limits)?,
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

/// Joint-model identity, physical point and known/declared-zero assumptions survive both channels.
/// ´test:integration:lc-frequency-model-and-point-retention´
#[test]
fn lc_frequency_model_and_point_retention() -> TestResult {
    let prepared = prepared()?;
    models::<f32>(&prepared)?;
    models::<f64>(&prepared)
}

fn models<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let half = P::ONE / P::from_u8(2);
    let quarter = P::ONE / P::from_u8(4);
    for (factor, expected, zero) in [
        ([half, quarter, P::ZERO, P::ZERO], 576, false),
        ([half, P::ZERO, P::ZERO, quarter], 320, true),
        ([half, P::ZERO, P::ZERO, P::ZERO], 64, true),
        ([P::ZERO; 4], 0, true),
    ] {
        let cross = if zero {
            CovarianceBlock::DeclaredZero("explicit native frequency zero block".into())
        } else {
            CovarianceBlock::Known
        };
        let mut budget = shared::policy()?;
        let point = [quarter, P::ONE / P::from_u8(16)];
        let mut case = case(
            prepared,
            point.map(|value| [value, P::ZERO, P::ZERO]),
            factor,
            [P::ONE; 2],
            cross.clone(),
            &mut budget,
        )?;
        let model = case.heap.statistical(case.companion)?.model().id();
        let sources = case
            .heap
            .statistical(case.companion)?
            .model()
            .sources()
            .iter()
            .map(Source::id)
            .collect::<Vec<_>>();
        let plan = publication(prepared, &case, &mut budget)?;
        let context = ArithmeticContext::new()?;
        let output = case
            .heap
            .publish_lc_frequency(&plan, &context, &mut budget)?;
        let view = output.source_model(&case.heap, &context)?;
        assert_eq!(view.model().id(), model);
        assert_eq!(
            view.model()
                .sources()
                .iter()
                .map(Source::id)
                .collect::<Vec<_>>(),
            sources
        );
        assert_eq!(view.model().block(0, 1)?, &cross);
        assert_eq!(view.source_maps(), &[vec![0], vec![1]]);
        assert!(view.means().is_none());
        assert_eq!(output.operating_point(&case.heap, &context)?, point);
        assert_eq!(
            output.assurance(&case.heap, &context)?,
            nrz_matrix::MomentAssurance::LinearizedMoments
        );
        let covariance = output.covariance(&case.heap, &context)?;
        assert!(
            crate::reciprocal_testing::oracle::Rational::new(expected, 1)?.covered(
                covariance[0][0].center.to_f64(),
                covariance[0][0].error.to_f64()
            )?
        );
        super::numeric::scaled_covariance(
            covariance,
            &crate::reciprocal_testing::oracle::Rational::new(expected, 1)?,
        )?;
    }
    Ok(())
}

/// Unknown source blocks are retained as unknown and cannot authorize a connected covariance output.
/// ´test:integration:lc-frequency-unknown-block-refusal´
#[test]
fn lc_frequency_unknown_block_refusal() -> TestResult {
    let prepared = prepared()?;
    unknown::<f32>(&prepared)?;
    unknown::<f64>(&prepared)
}

fn unknown<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    let half = P::ONE / P::from_u8(2);
    let quarter = half / P::from_u8(2);
    let case = case(
        prepared,
        [
            [quarter, P::ZERO, P::ZERO],
            [quarter * quarter, P::ZERO, P::ZERO],
        ],
        [half, quarter, P::ZERO, P::ZERO],
        [P::ONE; 2],
        CovarianceBlock::Unknown,
        &mut budget,
    )?;
    let before = case.heap.provenance_usage();
    assert!(publication(prepared, &case, &mut budget).is_err());
    assert_eq!(case.heap.provenance_usage(), before);
    assert_eq!(
        case.heap.statistical(case.companion)?.model().block(0, 1)?,
        &CovarianceBlock::Unknown
    );
    Ok(())
}

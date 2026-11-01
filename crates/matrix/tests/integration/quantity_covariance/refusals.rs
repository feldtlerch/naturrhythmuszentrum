// SPDX-License-Identifier: AGPL-3.0-only
//! Missing source and cross-block evidence remains an atomic statistical refusal.

use super::{Pair, TestResult, pair};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_matrix::{
    CompanionSetDraft, CovarianceBlock, DenseCovariance, ExecutionLimits, Extent, Heap, HeapLimits,
    JointError, JointLimits, JointModel, OperationError, QuantityContext, QuantityError,
    ShapeLimits, Source, StatisticsError, TargetMap,
};
use nrz_scalar::ArithmeticContext;

/// The three distinct missing-evidence contracts exercised beside a complete-model twin.
#[derive(Clone, Copy)]
enum Missing {
    /// No statistical source is declared for the right operand.
    Source,
    /// Each operand has its own unrelated model.
    Model,
    /// One shared model explicitly leaves its cross block unknown.
    Cross,
}

/// Declare only the evidence selected by the refusal case.
fn incomplete(missing: Missing) -> TestResult<Pair<f64>> {
    let context = QuantityContext::new(
        crate::kernel_testing::owner()?,
        ShapeLimits::default(),
        ExecutionLimits::default(),
    );
    let mut heap = Heap::new(HeapLimits::default())?;
    let ratio = Descriptor::ratio();
    let left = heap
        .import_represented(Extent::new(1, 1)?, &ratio, &[3.0_f64])?
        .value();
    let right = heap
        .import_represented(Extent::new(1, 1)?, &ratio, &[2.0_f64])?
        .value();
    match missing {
        Missing::Source | Missing::Model => {
            crate::statistics_testing::attach(&mut heap, left, &[0.25], false)?;
            if matches!(missing, Missing::Model) {
                crate::statistics_testing::attach(&mut heap, right, &[0.5], false)?;
            }
        }
        Missing::Cross => {
            let limits = JointLimits::default();
            let sources = [Source::new(&ratio, 1)?, Source::new(&ratio, 1)?];
            let covariance = DenseCovariance::new(
                2,
                &[0.25, 0.0, 0.0, 0.5],
                &ArithmeticContext::new()?,
                limits,
            )?;
            let model = JointModel::from_dense(
                &sources,
                covariance,
                &[
                    CovarianceBlock::Known,
                    CovarianceBlock::Unknown,
                    CovarianceBlock::Unknown,
                    CovarianceBlock::Known,
                ],
                limits,
            )?;
            for (index, handle) in [left, right].into_iter().enumerate() {
                let target = TargetMap::capture(&heap, handle)?;
                heap.publish_joint(
                    CompanionSetDraft::new(
                        model.clone(),
                        [target.clone(), target],
                        [vec![index], vec![index]],
                    ),
                    limits,
                )?;
            }
        }
    }
    let first = context.capture(
        &heap,
        heap.symbol_binding(left)?,
        &mut Budget::new(Limits::default())?,
    )?;
    let second = context.capture(
        &heap,
        heap.symbol_binding(right)?,
        &mut Budget::new(Limits::default())?,
    )?;
    Ok(Pair {
        context,
        heap,
        values: [first, second],
        handles: [left, right],
    })
}

/// Absence and incompatibility cannot silently turn correlated quantities into independent inputs.
/// ´test:integration:quantity-product-covariance-source-refusals´
/// ´claim:matrix:quantity-product-covariance-source-refusals´ Missing operand sources, unrelated models and unknown cross blocks refuse atomically with their exact named causes; complete correlated twins publish in fresh stores.
#[test]
fn quantity_product_covariance_source_refusals() -> TestResult {
    for missing in [Missing::Source, Missing::Model, Missing::Cross] {
        let mut case = incomplete(missing)?;
        let before = case.heap.provenance_usage();
        let mut budget = Budget::new(Limits::default())?;
        let Err(fault) = case.context.multiply(
            &mut case.heap,
            &case.values[0],
            &case.values[1],
            &mut budget,
        ) else {
            panic!("incomplete quantity covariance must refuse");
        };
        let QuantityError::Operation(fault) = fault else {
            panic!("statistical preflight refusal required");
        };
        match missing {
            Missing::Source => assert!(matches!(
                *fault,
                OperationError::Statistics(StatisticsError::MissingSource { slot: 1, entry: 0 })
            )),
            Missing::Model => assert!(matches!(
                *fault,
                OperationError::Statistics(StatisticsError::MissingJointModel)
            )),
            Missing::Cross => assert!(matches!(
                *fault,
                OperationError::Statistics(StatisticsError::Joint(JointError::MissingCrossBlock))
            )),
        }
        assert_eq!(case.heap.provenance_usage(), before);
        assert!(budget.usage().work > 0);
        let ratio = Descriptor::ratio();
        let mut twin = pair::<f64>([&ratio, &ratio])?;
        twin.context.multiply(
            &mut twin.heap,
            &twin.values[0],
            &twin.values[1],
            &mut Budget::new(Limits::default())?,
        )?;
    }
    Ok(())
}

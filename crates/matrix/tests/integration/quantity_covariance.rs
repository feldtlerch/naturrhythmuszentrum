// SPDX-License-Identifier: AGPL-3.0-only
//! Quantity covariance retains exact coordinate derivatives and complete joint sources.

mod boundaries;
mod physical;
mod refusals;

use crate::kernel_oracle::Rational;
use crate::kernel_testing::{TestResult, owner};
use crate::statistics_testing::check;
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_matrix::{
    CompanionSetDraft, CovarianceBlock, DenseCovariance, ExecutionLimits, Extent, Handle, Heap,
    HeapLimits, JointLimits, JointModel, MomentAssurance, Quantity, QuantityContext, ShapeLimits,
    Source, TargetMap, Value,
};
use nrz_scalar::{ArithmeticContext, Scalar};

/// A complete pair with a nonzero cross term and independently represented coordinates.
struct Pair<P: Scalar> {
    /// One frozen physical rule owner.
    context: QuantityContext,
    /// Ordinary source, output and moment storage.
    heap: Heap,
    /// Original selected operands.
    values: [Quantity<P>; 2],
    /// Original numerical handles used to test source invalidation.
    handles: [Handle<Value, P>; 2],
}

/// The exact joint matrix is `[[1/4,1/8],[1/8,1/2]]` at coordinates `(3,2)`.
fn pair<P: Scalar + 'static>(descriptors: [&Descriptor; 2]) -> TestResult<Pair<P>> {
    let context =
        QuantityContext::new(owner()?, ShapeLimits::default(), ExecutionLimits::default());
    append_pair(
        context,
        Heap::new(HeapLimits::default())?,
        descriptors,
        [P::from_u8(3), P::from_u8(2)],
    )
}

/// Append fresh independent numerical selections while optionally retaining the same rule authority.
fn append_pair<P: Scalar + 'static>(
    context: QuantityContext,
    mut heap: Heap,
    descriptors: [&Descriptor; 2],
    centers: [P; 2],
) -> TestResult<Pair<P>> {
    let first = heap.import_represented(
        Extent::new(1, 1)?,
        descriptors[0],
        std::slice::from_ref(&centers[0]),
    )?;
    let second = heap.import_represented(
        Extent::new(1, 1)?,
        descriptors[1],
        std::slice::from_ref(&centers[1]),
    )?;
    let limits = JointLimits::default();
    let sources = [
        Source::new(descriptors[0], 1)?,
        Source::new(descriptors[1], 1)?,
    ];
    let covariance = DenseCovariance::new(
        2,
        &[
            P::ONE / P::from_u8(4),
            P::ONE / P::from_u8(8),
            P::ONE / P::from_u8(8),
            P::ONE / P::from_u8(2),
        ],
        &ArithmeticContext::new()?,
        limits,
    )?;
    let model = JointModel::from_dense(
        &sources,
        covariance,
        &[const { CovarianceBlock::Known }; 4],
        limits,
    )?;
    let targets = [
        TargetMap::capture(&heap, first.value())?,
        TargetMap::capture(&heap, second.value())?,
    ];
    heap.publish_joint(
        CompanionSetDraft::new(model, targets, [vec![0], vec![1]]),
        limits,
    )?;
    let values = [first.value(), second.value()].map(|handle| {
        context
            .capture(
                &heap,
                heap.symbol_binding(handle)?,
                &mut Budget::new(Limits::default())?,
            )
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
    });
    let [left, right] = values;
    Ok(Pair {
        context,
        heap,
        values: [left?, right?],
        handles: [first.value(), second.value()],
    })
}

/// The independent product variance includes the complete original cross term.
/// ´test:integration:quantity-product-covariance-oracle´
/// ´claim:matrix:quantity-product-covariance-oracle´ Correlated quantity products enclose the independent rational variance seven and retain their coordinate Jacobian, source model and linearized assurance in both native formats.
#[test]
fn quantity_product_covariance_oracle() -> TestResult {
    product::<f32>()?;
    product::<f64>()
}

/// Compare the generic statistical contraction with a separately expanded rational law.
fn product<P: Scalar + 'static>() -> TestResult {
    let ratio = Descriptor::ratio();
    let mut case = pair::<P>([&ratio, &ratio])?;
    let result = case.context.multiply(
        &mut case.heap,
        &case.values[0],
        &case.values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    let handle = case
        .heap
        .companion(result.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = case.heap.moments(case.heap.moment_companion(handle)?)?;
    let variance = Rational::new(2, 1)?
        .multiply(Rational::new(2, 1)?)?
        .multiply(Rational::new(1, 4)?)?
        .add(
            Rational::new(3, 1)?
                .multiply(Rational::new(3, 1)?)?
                .multiply(Rational::new(1, 2)?)?,
        )?
        .add(Rational::new(12, 1)?.multiply(Rational::new(1, 8)?)?)?;
    check(moments.covariance(), &[variance])?;
    check(
        moments.jacobian().ok_or("Jacobian")?,
        &[Rational::new(2, 1)?, Rational::new(3, 1)?],
    )?;
    assert_eq!(moments.assurance(), MomentAssurance::LinearizedMoments);
    assert!(moments.means().is_none());
    let proof = moments.checked_sensitivity().ok_or("checked sensitivity")?;
    assert_eq!(
        proof.recipe(),
        nrz_bindings::SensitivityRecipe::QuantityProduct
    );
    assert_eq!(proof.outputs(), std::slice::from_ref(result.descriptor()));
    assert_eq!(
        proof.inputs(),
        [
            case.values[0].descriptor().clone(),
            case.values[1].descriptor().clone()
        ]
    );
    assert_eq!(proof.jacobian().witnesses().len(), 2);
    assert!(
        proof
            .jacobian()
            .entries()
            .iter()
            .all(|entry| entry.identity().is_discharged())
    );
    let repeated = case.context.multiply(
        &mut case.heap,
        &case.values[0],
        &case.values[0],
        &mut Budget::new(Limits::default())?,
    )?;
    let handle = case
        .heap
        .companion(repeated.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    check(
        case.heap
            .moments(case.heap.moment_companion(handle)?)?
            .covariance(),
        &[Rational::new(9, 1)?],
    )?;
    case.heap.compact()?;
    let version = case.heap.value(case.handles[0])?.stamp().version();
    case.heap
        .replace_represented(case.handles[0], version, &[P::from_u8(3)])?;
    let Err(fault) = result.view(&case.heap) else {
        panic!("changed original quantity source must refuse");
    };
    assert_eq!(fault, nrz_matrix::HeapError::StaleEvidence);
    Ok(())
}

/// Noncoherent coordinate scales belong in descriptors rather than in the stored-coordinate Jacobian.
/// ´test:integration:quantity-product-covariance-measures´
/// ´claim:matrix:quantity-product-covariance-measures´ Noncoherent products retain the exact represented operating points, zero local Jacobian error and independent rational covariance including original cross correlation and certified conversion-coefficient error.
#[test]
fn quantity_product_covariance_measures() -> TestResult {
    let first = Descriptor::new(
        nrz_bindings::Kind::Ratio,
        crate::conversion_testing::measure(2, 1, 0, 1)?,
        nrz_bindings::Role::Difference,
    )?;
    let second = Descriptor::new(
        nrz_bindings::Kind::Ratio,
        crate::conversion_testing::measure(3, 1, 0, 1)?,
        nrz_bindings::Role::Difference,
    )?;
    let ratio = Descriptor::ratio();
    let context =
        QuantityContext::new(owner()?, ShapeLimits::default(), ExecutionLimits::default());
    let mut case = append_pair(
        context,
        Heap::new(HeapLimits::default())?,
        [&ratio, &ratio],
        [6.0_f64, 6.0],
    )?;
    let left = case.context.convert(
        &mut case.heap,
        &case.values[0],
        &first,
        &mut Budget::new(Limits::default())?,
    )?;
    let right = case.context.convert(
        &mut case.heap,
        &case.values[1],
        &second,
        &mut Budget::new(Limits::default())?,
    )?;
    let result = case.context.multiply(
        &mut case.heap,
        &left,
        &right,
        &mut Budget::new(Limits::default())?,
    )?;
    check_measures(&case.heap, &left, &right, &result)?;
    let expected = case.context.declarations().registry().coordinate_binary(
        nrz_bindings::BinaryOp::Multiply,
        &first,
        &second,
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(result.descriptor(), &expected);
    Ok(())
}

/// Decode actual operand bits and independently expand the exact source-coordinate variance.
fn check_measures(
    heap: &Heap,
    left: &Quantity<f64>,
    right: &Quantity<f64>,
    result: &Quantity<f64>,
) -> TestResult {
    let x = Rational::binary(left.view(heap)?.region().get(0, 0)?)?;
    let right_view = right.view(heap)?;
    let right_center = right_view.region().get(0, 0)?;
    let y = Rational::binary(right_center)?;
    assert!(right_center < 2.0);
    let input_error = heap
        .companion(right_view.evaluation().ok_or("conversion error")?)?
        .region()
        .get(0, 0)?;
    assert!(Rational::new(2, 1)?.covered(right_center, input_error)?);
    let a = y.multiply(Rational::new(1, 2)?)?;
    let b = x.multiply(Rational::new(1, 3)?)?;
    let variance = a
        .multiply(a)?
        .multiply(Rational::new(1, 4)?)?
        .add(b.multiply(b)?.multiply(Rational::new(1, 2)?)?)?
        .add(a.multiply(b)?.multiply(Rational::new(1, 4)?)?)?;
    let handle = heap
        .companion(result.view(heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = heap.moments(heap.moment_companion(handle)?)?;
    let jacobian = moments.jacobian().ok_or("Jacobian")?;
    check(jacobian, &[y, x])?;
    assert_eq!(jacobian.error(0, 0)?, 0.0);
    assert_eq!(jacobian.error(0, 1)?, 0.0);
    check(moments.sensitivity().ok_or("source sensitivity")?, &[a, b])?;
    check(moments.covariance(), &[variance])?;
    Ok(())
}

/// Immutable proof reuse never imports a stale operating point or source covariance.
/// ´test:integration:quantity-product-covariance-proof-reuse´
/// ´claim:matrix:quantity-product-covariance-proof-reuse´ Compatible fresh sources reuse a published product identity after its old numerical source becomes stale, while changed rule authority or non-Work symbolic policy constructs a distinct proof.
#[test]
fn quantity_product_covariance_proof_reuse() -> TestResult {
    let ratio = Descriptor::ratio();
    let case = pair::<f64>([&ratio, &ratio])?;
    let next = append_pair(
        case.context.clone(),
        case.heap,
        [&ratio, &ratio],
        [4.0, 3.0],
    )?;
    let foreign =
        QuantityContext::new(owner()?, ShapeLimits::default(), ExecutionLimits::default());
    let mut last = append_pair(foreign, next.heap, [&ratio, &ratio], [4.0, 3.0])?;
    let first = case.context.multiply(
        &mut last.heap,
        &case.values[0],
        &case.values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    let identity = proof_identity(&last.heap, &first)?;
    let version = last.heap.value(case.handles[0])?.stamp().version();
    last.heap
        .replace_represented(case.handles[0], version, &[3.0])?;
    let second = next.context.multiply(
        &mut last.heap,
        &next.values[0],
        &next.values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    assert_eq!(proof_identity(&last.heap, &second)?, identity);
    let handle = last
        .heap
        .companion(second.view(&last.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = last.heap.moments(last.heap.moment_companion(handle)?)?;
    check(moments.covariance(), &[Rational::new(53, 4)?])?;
    check(
        moments.jacobian().ok_or("Jacobian")?,
        &[Rational::new(3, 1)?, Rational::new(4, 1)?],
    )?;
    let third = last.context.multiply(
        &mut last.heap,
        &last.values[0],
        &last.values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    let foreign_identity = proof_identity(&last.heap, &third)?;
    assert_ne!(foreign_identity, identity);
    let defaults = Limits::default();
    let mut changed = Budget::new(Limits {
        replay_work: defaults.replay_work - 1,
        ..defaults
    })?;
    let fourth = last.context.multiply(
        &mut last.heap,
        &last.values[0],
        &last.values[1],
        &mut changed,
    )?;
    assert_ne!(proof_identity(&last.heap, &fourth)?, foreign_identity);
    Ok(())
}

/// Read only the immutable symbolic owner identity from ordinary published moments.
fn proof_identity(heap: &Heap, value: &Quantity<f64>) -> TestResult<nrz_bindings::SnapshotId> {
    let handle = heap
        .companion(value.view(heap)?.bound().ok_or("bound")?)?
        .target();
    Ok(heap
        .moments(heap.moment_companion(handle)?)?
        .checked_sensitivity()
        .ok_or("sensitivity")?
        .snapshot_id()
        .clone())
}

/// Product composition preserves repeated dependence on the original joint sources.
/// ´test:integration:quantity-product-covariance-composition´
/// ´claim:matrix:quantity-product-covariance-composition´ The quantity law `(x*y)*x` retains the original correlated source model, the composed derivative `(12,9)` and the independent rational variance `207/2`.
#[test]
fn quantity_product_covariance_composition() -> TestResult {
    let ratio = Descriptor::ratio();
    let mut case = pair::<f64>([&ratio, &ratio])?;
    let first = case.context.multiply(
        &mut case.heap,
        &case.values[0],
        &case.values[1],
        &mut Budget::new(Limits::default())?,
    )?;
    let first_target = case
        .heap
        .companion(first.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let original_model = case
        .heap
        .moments(case.heap.moment_companion(first_target)?)?
        .model();
    let second = case.context.multiply(
        &mut case.heap,
        &first,
        &case.values[0],
        &mut Budget::new(Limits::default())?,
    )?;
    let second_target = case
        .heap
        .companion(second.view(&case.heap)?.bound().ok_or("bound")?)?
        .target();
    let moments = case
        .heap
        .moments(case.heap.moment_companion(second_target)?)?;
    assert_eq!(moments.model(), original_model);
    assert_eq!(moments.assurance(), MomentAssurance::LinearizedMoments);
    assert!(moments.means().is_none());
    check(
        moments.sensitivity().ok_or("composed source derivative")?,
        &[Rational::new(12, 1)?, Rational::new(9, 1)?],
    )?;
    check(moments.covariance(), &[Rational::new(207, 2)?])?;
    Ok(())
}

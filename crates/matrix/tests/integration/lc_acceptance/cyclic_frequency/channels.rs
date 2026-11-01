// SPDX-License-Identifier: AGPL-3.0-only
//! Certified coefficient error propagates numerical channels without becoming physical variance.

use super::{Prepared, TestResult, conversion, inputs, prepared, publication, shared};
use crate::integration::pi_conversion::{exact_policy, oracle, pi};
use crate::reciprocal_testing::oracle::Rational;
use nrz_matrix::{
    CompanionSetDraft, Coverage, DependencyPolicy, Extent, Heap, HeapLimits, JointLimits,
    LcFrequencyPublication, ReadingRegion,
};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// Full deterministic and prior numerical intervals are transformed independently with one coefficient.
/// ´test:integration:lc-frequency-deterministic-numerical-channels´
#[test]
fn lc_frequency_deterministic_numerical_channels() -> TestResult {
    let prepared = prepared()?;
    channels::<f32>(&prepared)?;
    channels::<f64>(&prepared)
}

fn channels<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    let one = P::ONE;
    let mut case = inputs::prepare(
        &prepared.fixture,
        &prepared.derivative,
        [
            [
                one / P::from_u8(4),
                one / P::from_u8(64),
                one / (P::from_u8(128) * P::from_u8(8)),
            ],
            [
                one / P::from_u8(16),
                one / (P::from_u8(128) * P::from_u8(2)),
                one / (P::from_u8(128) * P::from_u8(32)),
            ],
        ],
        [
            one / P::from_u8(128),
            one / (P::from_u8(128) * P::from_u8(4)),
        ],
        true,
        &mut budget,
    )?;
    let initial = publication(prepared, &case, &mut budget)?;
    let context = ArithmeticContext::new()?;
    let original = initial.evaluation().imaginary_rates(&case.heap, &context)?[0];
    let coefficient = conversion(prepared)?;
    let circle = pi::<P>(&context, &mut budget)?;
    let evidence = circle.evidence();
    let plan = LcFrequencyPublication::new(
        initial.evaluation().clone(),
        prepared.definition.clone(),
        &coefficient,
        &circle,
        &exact_policy(),
        &context,
        &mut budget,
    )?;
    drop(initial);
    drop(coefficient);
    drop(circle);
    let output = case
        .heap
        .publish_lc_frequency(&plan, &context, &mut budget)?;
    let [angular, cyclic] = output.values(&case.heap, &context)?;
    assert_eq!(
        [angular.center, angular.total, angular.error],
        [original.center, original.total, original.error]
    );
    assert!(
        output
            .constant(&case.heap, &context)?
            .same_region(&evidence)
    );
    assert!(
        output
            .coefficient(&case.heap, &context)?
            .displacement()
            .get()
            > P::ZERO
    );
    for (radius, transformed) in [(angular.total, cyclic.total), (angular.error, cyclic.error)] {
        for sign in [-1.0, 1.0] {
            let target = Rational::binary(angular.center.to_f64())?
                .add(&Rational::binary(sign * radius.to_f64())?)?;
            for endpoint in oracle::factor(1, 2, true, 56)? {
                let factor = Rational::new(
                    i64::try_from(endpoint.numerator)?,
                    u64::try_from(endpoint.denominator)?,
                )?;
                assert!(
                    target
                        .multiply(&factor)?
                        .covered(cyclic.center.to_f64(), transformed.to_f64())?
                );
            }
        }
    }
    assert!(cyclic.error <= cyclic.total);
    assert!(cyclic.error > P::ZERO);
    Ok(())
}

/// Source-free conversion error cannot upgrade represented-input coverage or invent a statistical mean.
/// ´test:integration:lc-frequency-represented-coverage-retention´
#[test]
fn lc_frequency_represented_coverage_retention() -> TestResult {
    let prepared = prepared()?;
    represented::<f32>(&prepared)?;
    represented::<f64>(&prepared)
}

fn represented<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let regions = [P::ONE / P::from_u8(4), P::ONE / P::from_u8(16)]
        .into_iter()
        .map(|value| {
            Ok(ReadingRegion {
                extent: Extent::new(1, 1)?,
                values: vec![value],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    let imported = heap.import_readings(
        Rc::clone(&prepared.fixture.graph),
        prepared.fixture.coverage.clone(),
        Vec::new(),
        &regions,
        DependencyPolicy::WholeBase,
        &mut budget,
    )?;
    let original = imported
        .bindings()
        .to_vec()
        .try_into()
        .map_err(|_| "two represented inputs")?;
    let selected = super::super::invariants::selections(&mut heap, &original)?;
    let handles = super::super::selected_joint::capture_handles(&mut heap, &selected)?;
    let model = super::super::selected_joint::joint_model(
        &selected,
        &[P::ONE / P::from_u8(2), P::ONE / P::from_u8(4)],
    )?;
    let companion = heap.publish_joint(
        CompanionSetDraft::new(
            model,
            super::super::selected_joint::capture_targets(&heap, handles)?,
            [vec![0], vec![1]],
        ),
        JointLimits::default(),
    )?;
    let context = ArithmeticContext::new()?;
    let derivative = heap.evaluate_reciprocal_derivative(
        &prepared.derivative,
        &original,
        &context,
        &mut budget,
    )?;
    let mut case = inputs::Case {
        heap,
        original,
        selected,
        derivative,
        companion,
    };
    let plan = publication(prepared, &case, &mut budget)?;
    let output = case
        .heap
        .publish_lc_frequency(&plan, &context, &mut budget)?;
    assert_eq!(
        output.evaluation(&case.heap, &context)?.coverage(),
        [Coverage::RepresentedInputs; 2]
    );
    for reading in output.readings().bindings() {
        assert_eq!(
            reading.phasor_input_coverage(),
            Some([Coverage::RepresentedInputs; 2])
        );
    }
    assert!(output.source_model(&case.heap, &context)?.means().is_none());
    let cyclic = output.values(&case.heap, &context)?[1];
    assert!(cyclic.error > P::ZERO);
    assert_eq!(cyclic.error, cyclic.total);
    oracle::covers(
        oracle::factor(4, 1, true, 56)?,
        cyclic.center.to_f64(),
        cyclic.error.to_f64(),
    )?;
    Ok(())
}

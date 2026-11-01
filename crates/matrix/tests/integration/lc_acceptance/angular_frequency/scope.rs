// SPDX-License-Identifier: AGPL-3.0-only
//! Statistical publication preserves the original physical scope and its heap-owned provenance.

use super::super::selected_joint;
use super::{TestResult, prepared, shared};
use nrz_matrix::{
    CompanionSetDraft, Coverage, Heap, HeapLimits, JointLimits, LcAngularFrequencyPublication,
    MomentAssurance,
};
use nrz_scalar::ArithmeticContext;

/// ´test:integration:lc-angular-frequency-retains-original-scope-and-owner´
#[test]
fn lc_angular_frequency_retains_original_scope_and_owner() -> TestResult {
    let prepared = prepared()?;
    let mut budget = shared::policy()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let original = super::super::represented(&prepared.fixture, &mut heap)?;
    let selected = super::super::invariants::selections(&mut heap, &original)?;
    let handles = selected_joint::capture_handles(&mut heap, &selected)?;
    let model = selected_joint::joint_model(&selected, &[0.5, 0.25])?;
    let companion = heap.publish_joint(
        CompanionSetDraft::new(
            model,
            selected_joint::capture_targets(&heap, handles)?,
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
    let spectrum = prepared
        .fixture
        .plan(original.clone(), &mut budget)?
        .with_reciprocal_statistics(&heap, derivative, companion, &mut budget)?;
    let plan =
        LcAngularFrequencyPublication::new(spectrum, prepared.definition.clone(), &mut budget)?;
    let output = heap.publish_lc_angular_frequency(&plan, &context, &mut budget)?;
    assert_eq!(
        output.evaluation().coverage(),
        [Coverage::RepresentedInputs; 2]
    );
    assert_eq!(
        output.assurance(&heap, &context)?,
        MomentAssurance::LinearizedMoments
    );
    assert!(output.source_model(&heap, &context)?.means().is_none());
    let reading = output.readings().bindings()[0].clone();
    assert_eq!(
        reading.phasor_input_coverage(),
        Some([Coverage::RepresentedInputs; 2])
    );
    let value = output.value(&heap, &context)?;
    assert_eq!(heap.reading_entries(&reading)?, vec![value.center]);
    drop(output);
    drop(plan);
    heap.compact()?;
    assert!(
        reading
            .lc_angular_frequency()
            .ok_or("heap-owned original recipe")?
            .law()
            .shares_allocation(&prepared.fixture.law)
    );
    assert_eq!(heap.reading_entries(&reading)?, vec![value.center]);
    let values = heap.reading_entries(&original[0])?;
    heap.replace_reading_entries(&original[0], &values, &mut budget)?;
    assert!(heap.reading_entries(&reading).is_err());
    Ok(())
}

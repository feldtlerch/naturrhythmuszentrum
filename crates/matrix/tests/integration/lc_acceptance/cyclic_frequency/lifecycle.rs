// SPDX-License-Identifier: AGPL-3.0-only
//! Every original and produced channel shares the connected publication's retirement boundary.

use super::{Prepared, TestResult, inputs, prepared, publication, shared};
use nrz_matrix::{Heap, PublishedLcFrequency};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Equal-value writes to all six source channels and all twelve produced channels retire everything.
/// ´test:integration:lc-frequency-all-channel-retirement´
#[test]
fn lc_frequency_all_channel_retirement() -> TestResult {
    let prepared = prepared()?;
    retirement::<f32>(&prepared)?;
    retirement::<f64>(&prepared)
}

fn retirement<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    for changed in 0..18 {
        let mut budget = shared::policy()?;
        let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut budget)?;
        let plan = publication(prepared, &case, &mut budget)?;
        let context = ArithmeticContext::new()?;
        let output = case
            .heap
            .publish_lc_frequency(&plan, &context, &mut budget)?;
        match changed {
            0..=5 => super::super::invariants::mutate(&mut case.heap, &case.original, changed)?,
            6..=10 => {
                let reading = &output.readings().bindings()[changed - 6];
                let values = case.heap.reading_entries(reading)?;
                case.heap
                    .replace_reading_entries(reading, &values, &mut budget)?;
            }
            11..=17 => {
                let index = changed - 11;
                let reading =
                    &output.readings().bindings()[if index < 3 || index == 6 { 0 } else { 2 }];
                let values = case.heap.reading_companion_entries(reading, index)?;
                case.heap.replace_reading_companion_entries(
                    reading,
                    index,
                    &values,
                    &mut budget,
                )?;
            }
            _ => return Err("frequency channel mutation".into()),
        }
        unavailable(&output, &case.heap, &context);
    }
    Ok(())
}

fn unavailable<P: Scalar + 'static>(
    output: &PublishedLcFrequency<P>,
    heap: &Heap,
    context: &ArithmeticContext,
) {
    assert!(output.values(heap, context).is_err());
    assert!(output.evaluation(heap, context).is_err());
    assert!(output.operating_point(heap, context).is_err());
    assert!(output.jacobians(heap, context).is_err());
    assert!(output.covariance(heap, context).is_err());
    assert!(output.assurance(heap, context).is_err());
    assert!(output.source_model(heap, context).is_err());
    assert!(output.coefficient(heap, context).is_err());
    assert!(output.constant(heap, context).is_err());
    for reading in output.readings().bindings() {
        assert!(heap.reading_entries(reading).is_err());
    }
}

/// The heap owns the recipe after wrapper disposal, and source retirement still invalidates both values.
/// ´test:integration:lc-frequency-heap-owned-origin-retirement´
#[test]
fn lc_frequency_heap_owned_origin_retirement() -> TestResult {
    let prepared = prepared()?;
    retained::<f32>(&prepared)?;
    retained::<f64>(&prepared)
}

fn retained<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut budget)?;
    let handles = super::super::selected_joint::capture_handles(&mut case.heap, &case.selected)?;
    let plan = publication(prepared, &case, &mut budget)?;
    let context = ArithmeticContext::new()?;
    let output = case
        .heap
        .publish_lc_frequency(&plan, &context, &mut budget)?;
    let readings = output.readings().clone();
    drop(output);
    drop(plan);
    case.heap.compact()?;
    for reading in readings.bindings() {
        assert!(
            reading
                .lc_frequency()
                .ok_or("heap-owned frequency recipe")?
                .angular()
                .law()
                .shares_allocation(&prepared.fixture.law)
        );
        case.heap.reading_entries(reading)?;
    }
    case.heap.retire(handles[0])?;
    for reading in readings.bindings() {
        assert!(case.heap.reading_entries(reading).is_err());
    }
    Ok(())
}

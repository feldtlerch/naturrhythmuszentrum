// SPDX-License-Identifier: AGPL-3.0-only
//! Equal-value writes to any original or published parameter retire the complete connected output.

use super::{TestResult, inputs, prepared, publication, shared};
use nrz_scalar::{ArithmeticContext, Scalar};

/// ´test:integration:lc-angular-frequency-source-companion-retirement´
#[test]
fn lc_angular_frequency_source_companion_retirement() -> TestResult {
    let prepared = prepared()?;
    retirement::<f32>(&prepared)?;
    retirement::<f64>(&prepared)
}

fn retirement<P: Scalar + 'static>(prepared: &super::Prepared) -> TestResult {
    for changed in 0..11 {
        let mut budget = shared::policy()?;
        let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut budget)?;
        let plan = publication(prepared, &case, &mut budget)?;
        let context = ArithmeticContext::new()?;
        let output = case
            .heap
            .publish_lc_angular_frequency(&plan, &context, &mut budget)?;
        match changed {
            0..=5 => super::super::invariants::mutate(&mut case.heap, &case.original, changed)?,
            6 | 7 => {
                let binding = &output.readings().bindings()[changed - 6];
                let values = case.heap.reading_entries(binding)?;
                case.heap
                    .replace_reading_entries(binding, &values, &mut budget)?;
            }
            8..=10 => {
                let binding = &output.readings().bindings()[0];
                let values = case.heap.reading_companion_entries(binding, changed - 8)?;
                case.heap.replace_reading_companion_entries(
                    binding,
                    changed - 8,
                    &values,
                    &mut budget,
                )?;
            }
            _ => return Err("mutation".into()),
        }
        assert!(output.value(&case.heap, &context).is_err());
        assert!(output.jacobian(&case.heap, &context).is_err());
        assert!(output.covariance(&case.heap, &context).is_err());
        assert!(output.assurance(&case.heap, &context).is_err());
        assert!(output.source_model(&case.heap, &context).is_err());
    }
    Ok(())
}

/// ´test:integration:lc-angular-frequency-positive-whole-box-refusals´
#[test]
fn lc_angular_frequency_positive_whole_box_refusals() -> TestResult {
    let prepared = prepared()?;
    whole_box::<f32>(&prepared)?;
    whole_box::<f64>(&prepared)
}

fn whole_box<P: Scalar + 'static>(prepared: &super::Prepared) -> TestResult {
    let quarter = P::ONE / P::from_u8(4);
    let sixteenth = P::ONE / P::from_u8(16);
    let half = P::ONE / P::from_u8(2);
    for (radii, input) in [
        ([quarter, P::ZERO], 0),
        ([P::ZERO, sixteenth], 1),
        ([half, P::ZERO], 0),
    ] {
        let values = [[quarter, radii[0], P::ZERO], [sixteenth, radii[1], P::ZERO]];
        let mut budget = shared::policy()?;
        let mut case = inputs::prepare(
            &prepared.fixture,
            &prepared.derivative,
            values,
            [half, quarter],
            true,
            &mut budget,
        )?;
        let plan = publication(prepared, &case, &mut budget)?;
        let before = case.heap.provenance_usage();
        let mut execution = shared::policy()?;
        let result = case.heap.publish_lc_angular_frequency(
            &plan,
            &ArithmeticContext::new()?,
            &mut execution,
        );
        assert!(
            matches!(result, Err(nrz_matrix::SpectralError::InputNotSeparated {input: actual}) if actual == input)
        );
        assert!(execution.usage().work > 0);
        assert_eq!(case.heap.provenance_usage(), before);
    }
    let mut budget = shared::policy()?;
    let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut budget)?;
    let plan = publication(prepared, &case, &mut budget)?;
    case.heap
        .publish_lc_angular_frequency(&plan, &ArithmeticContext::new()?, &mut budget)?;
    Ok(())
}
